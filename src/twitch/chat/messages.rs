use super::ffi::{G_SOURCE_REMOVE, GError, g_main_context_invoke_full};
use super::worker::error_message;
use super::{
    ChatLineData, TwitchChatClient, TwitchChatLine, twitch_chat_client_ref,
    twitch_chat_client_unref,
};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::ptr;

pub(super) const CHAT_MAINLOOP_PRIORITY: c_int = 300;

pub(super) const CHAT_MAX_PENDING_MAIN_LINES: usize = 128;

pub(super) const TWITCH_CHAT_LINE_STATUS: c_int = 0;

pub(super) const TWITCH_CHAT_LINE_MESSAGE: c_int = 1;

pub(super) unsafe fn prefixed_message(prefix: &[u8], detail: *mut GError) -> Vec<u8> {
    unsafe {
        let message = error_message(detail);
        let mut value = Vec::with_capacity(prefix.len() + message.len() + 1);
        value.extend_from_slice(prefix);
        value.extend_from_slice(&message);
        value.push(0);
        value
    }
}

pub(super) unsafe fn channel_message(prefix: &[u8], channel: *const c_char) -> Vec<u8> {
    unsafe {
        let channel = CStr::from_ptr(channel).to_bytes();
        let mut value = Vec::with_capacity(prefix.len() + channel.len() + 1);
        value.extend_from_slice(prefix);
        value.extend_from_slice(channel);
        value.push(0);
        value
    }
}

pub(super) unsafe extern "C" fn chat_line_data_free(data: *mut c_void) {
    unsafe {
        let data = data as *mut ChatLineData;
        if data.is_null() {
            return;
        }

        if (*data).counted_pending {
            (*(*data).client)
                .pending_lines
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }
        twitch_chat_client_unref((*data).client);
        drop(Box::from_raw(data));
    }
}

// Limit pending messages before handing them to the main context.
pub(super) unsafe fn queue_line_on_main(data: *mut ChatLineData) {
    unsafe {
        let previous_pending = (*(*data).client)
            .pending_lines
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        (*data).counted_pending = true;

        if previous_pending >= CHAT_MAX_PENDING_MAIN_LINES {
            chat_line_data_free(data as *mut c_void);
            return;
        }

        g_main_context_invoke_full(
            ptr::null_mut(),
            CHAT_MAINLOOP_PRIORITY,
            Some(emit_line_on_main),
            data as *mut c_void,
            Some(chat_line_data_free),
        );
    }
}

// Drop queued messages from a closed client or an earlier channel.
pub(super) unsafe extern "C" fn emit_line_on_main(user_data: *mut c_void) -> c_int {
    unsafe {
        let data = user_data as *mut ChatLineData;
        let callback = {
            let state = (*(*data).client).state.lock().expect("chat state poisoned");
            if state.closed || (*data).generation != state.generation {
                None
            } else {
                state.line_func.map(|callback| (callback, state.user_data))
            }
        };

        if let Some((line_func, line_user_data)) = callback {
            line_func(&(*data).line, line_user_data);
        }

        G_SOURCE_REMOVE
    }
}

pub(super) unsafe fn emit_status(
    client: *mut TwitchChatClient,
    generation: c_uint,
    message: *const c_char,
) {
    unsafe {
        let line = TwitchChatLine {
            kind: TWITCH_CHAT_LINE_STATUS,
            message: if message.is_null() {
                CString::default()
            } else {
                CStr::from_ptr(message).to_owned()
            },
            ..Default::default()
        };
        emit_message(client, generation, line);
    }
}

pub(super) unsafe fn emit_message(
    client: *mut TwitchChatClient,
    generation: c_uint,
    line: TwitchChatLine,
) {
    unsafe {
        let data = Box::into_raw(Box::new(ChatLineData {
            client: twitch_chat_client_ref(client),
            generation,
            line,
            counted_pending: false,
        }));
        queue_line_on_main(data);
    }
}

pub(super) fn extract_irc_tag_bytes(tags: &[u8], key: &[u8]) -> CString {
    for segment in tags.split(|byte| *byte == b';') {
        if let Some(raw) = segment
            .strip_prefix(key)
            .and_then(|tail| tail.strip_prefix(b"="))
            && !raw.is_empty()
        {
            let mut decoded = Vec::with_capacity(raw.len());
            let mut i = 0usize;
            while i < raw.len() {
                if raw[i] == b'\\' && i + 1 < raw.len() {
                    i += 1;
                    decoded.push(match raw[i] {
                        b's' => b' ',
                        b':' => b';',
                        b'\\' => b'\\',
                        b'r' => b'\r',
                        b'n' => b'\n',
                        other => other,
                    });
                } else {
                    decoded.push(raw[i]);
                }
                i += 1;
            }

            decoded.truncate(
                decoded
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(decoded.len()),
            );
            return CString::new(decoded).unwrap();
        }
    }

    CString::default()
}

pub(super) fn extract_sender_from_prefix(bytes: &[u8]) -> CString {
    bytes
        .iter()
        .position(|byte| *byte == b':')
        .and_then(|colon| {
            bytes[colon + 1..]
                .split(|byte| *byte == b'!')
                .next()
                .filter(|name| !name.is_empty() && bytes[colon + 1..].contains(&b'!'))
        })
        .map_or_else(
            || c"chat".into(),
            |name| CString::new(name).unwrap_or_default(),
        )
}

// Parse a complete IRC PRIVMSG before handing ownership to the UI queue.
pub(super) fn parse_privmsg(bytes: &[u8]) -> Option<TwitchChatLine> {
    let privmsg_start = bytes.windows(9).position(|window| window == b" PRIVMSG ")?;
    let rest = &bytes[privmsg_start + 9..];
    let trailing = rest.windows(2).position(|window| window == b" :")?;
    let message_start = privmsg_start + 9 + trailing + 2;
    let tags = if bytes.first() == Some(&b'@') {
        bytes
            .iter()
            .position(|byte| *byte == b' ')
            .map_or(&[][..], |end| &bytes[1..end])
    } else {
        &[]
    };
    let name = extract_irc_tag_bytes(tags, b"display-name");
    Some(TwitchChatLine {
        kind: TWITCH_CHAT_LINE_MESSAGE,
        display_name: if name.is_empty() {
            extract_sender_from_prefix(bytes)
        } else {
            name
        },
        message: CString::new(bytes[message_start..].trim_ascii_end()).ok()?,
        color: extract_irc_tag_bytes(tags, b"color"),
        emotes: extract_irc_tag_bytes(tags, b"emotes"),
        reply_display_name: extract_irc_tag_bytes(tags, b"reply-parent-display-name"),
        reply_message: extract_irc_tag_bytes(tags, b"reply-parent-msg-body"),
    })
}
