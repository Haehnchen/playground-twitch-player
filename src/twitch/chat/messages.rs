use super::ffi::{
    G_SOURCE_REMOVE, GError, g_atomic_int_add, g_free, g_main_context_invoke_full, g_mutex_lock,
    g_mutex_unlock, g_strdup,
};
use super::worker::error_message;
use super::{
    ChatLineData, ParsedPrivmsg, TwitchChatClient, TwitchChatLine, TwitchChatLineFunc,
    twitch_chat_client_ref, twitch_chat_client_unref,
};
use crate::glib_strings::dup_bytes;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::ptr;

pub(super) const CHAT_MAINLOOP_PRIORITY: c_int = 300;

pub(super) const CHAT_MAX_PENDING_MAIN_LINES: c_int = 128;

pub(super) const TWITCH_CHAT_LINE_STATUS: c_int = 0;

pub(super) const TWITCH_CHAT_LINE_MESSAGE: c_int = 1;

pub(super) unsafe fn dup_ptr(value: *const c_char) -> *mut c_char {
    unsafe {
        if value.is_null() {
            ptr::null_mut()
        } else {
            g_strdup(value)
        }
    }
}

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

        g_free((*data).display_name as *mut c_void);
        g_free((*data).message as *mut c_void);
        g_free((*data).color as *mut c_void);
        g_free((*data).emotes as *mut c_void);
        g_free((*data).reply_display_name as *mut c_void);
        g_free((*data).reply_message as *mut c_void);
        if (*data).counted_pending != 0 {
            g_atomic_int_add(&mut (*(*data).client).pending_lines, -1);
        }
        twitch_chat_client_unref((*data).client);
        drop(Box::from_raw(data));
    }
}

// Limit pending messages before handing them to the main context.
pub(super) unsafe fn queue_line_on_main(data: *mut ChatLineData) {
    unsafe {
        let previous_pending = g_atomic_int_add(&mut (*(*data).client).pending_lines, 1);
        (*data).counted_pending = 1;

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
        let mut line_func: Option<TwitchChatLineFunc> = None;
        let mut line_user_data: *mut c_void = ptr::null_mut();

        g_mutex_lock(&mut (*(*data).client).mutex);
        if (*(*data).client).closed == 0
            && (*data).generation == (*(*data).client).generation
            && (*(*data).client).line_func.is_some()
        {
            line_func = (*(*data).client).line_func;
            line_user_data = (*(*data).client).user_data;
        }
        g_mutex_unlock(&mut (*(*data).client).mutex);

        if let Some(line_func) = line_func {
            (*data).line.display_name = (*data).display_name;
            (*data).line.message = (*data).message;
            (*data).line.color = (*data).color;
            (*data).line.emotes = (*data).emotes;
            (*data).line.reply_display_name = (*data).reply_display_name;
            (*data).line.reply_message = (*data).reply_message;
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
        let data = Box::into_raw(Box::new(ChatLineData {
            client: twitch_chat_client_ref(client),
            generation,
            line: TwitchChatLine {
                kind: TWITCH_CHAT_LINE_STATUS,

                display_name: ptr::null(),
                message: ptr::null(),
                color: ptr::null(),
                emotes: ptr::null(),
                reply_display_name: ptr::null(),
                reply_message: ptr::null(),
            },

            display_name: ptr::null_mut(),
            message: dup_ptr(message),
            color: ptr::null_mut(),
            emotes: ptr::null_mut(),
            reply_display_name: ptr::null_mut(),
            reply_message: ptr::null_mut(),

            counted_pending: 0,
        }));

        queue_line_on_main(data);
    }
}

pub(super) unsafe fn emit_message(
    client: *mut TwitchChatClient,
    generation: c_uint,
    message: *mut ParsedPrivmsg,
) {
    unsafe {
        let data = Box::into_raw(Box::new(ChatLineData {
            client: twitch_chat_client_ref(client),
            generation,
            line: TwitchChatLine {
                kind: TWITCH_CHAT_LINE_MESSAGE,

                display_name: ptr::null(),
                message: ptr::null(),
                color: ptr::null(),
                emotes: ptr::null(),
                reply_display_name: ptr::null(),
                reply_message: ptr::null(),
            },

            display_name: dup_ptr((*message).display_name),
            message: dup_ptr((*message).message),
            color: dup_ptr((*message).color),
            emotes: dup_ptr((*message).emotes),
            reply_display_name: dup_ptr((*message).reply_display_name),
            reply_message: dup_ptr((*message).reply_message),

            counted_pending: 0,
        }));

        queue_line_on_main(data);
    }
}

pub(super) fn extract_irc_tag_bytes(tags: &[u8], key: &[u8]) -> *mut c_char {
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

            return dup_bytes(&decoded);
        }
    }

    ptr::null_mut()
}

pub(super) unsafe fn extract_sender_from_prefix(line: *const c_char) -> *mut c_char {
    unsafe {
        let bytes = CStr::from_ptr(line).to_bytes();
        let Some(colon) = bytes.iter().position(|byte| *byte == b':') else {
            return dup_bytes(b"chat");
        };
        let prefix = &bytes[colon + 1..];
        let Some(bang) = prefix.iter().position(|byte| *byte == b'!') else {
            return dup_bytes(b"chat");
        };
        if bang == 0 {
            return dup_bytes(b"chat");
        }

        dup_bytes(&prefix[..bang])
    }
}

pub(super) unsafe fn parsed_privmsg_free(message: *mut ParsedPrivmsg) {
    unsafe {
        if message.is_null() {
            return;
        }

        g_free((*message).display_name as *mut c_void);
        g_free((*message).message as *mut c_void);
        g_free((*message).color as *mut c_void);
        g_free((*message).emotes as *mut c_void);
        g_free((*message).reply_display_name as *mut c_void);
        g_free((*message).reply_message as *mut c_void);
        drop(Box::from_raw(message));
    }
}

pub(super) unsafe fn parse_privmsg(line: *const c_char) -> *mut ParsedPrivmsg {
    unsafe {
        let bytes = CStr::from_ptr(line).to_bytes();
        let Some(privmsg_start) = bytes.windows(9).position(|window| window == b" PRIVMSG ") else {
            return ptr::null_mut();
        };
        let rest = &bytes[privmsg_start + 9..];
        let Some(trailing) = rest.windows(2).position(|window| window == b" :") else {
            return ptr::null_mut();
        };
        let message_start = privmsg_start + 9 + trailing + 2;

        let mut name: *mut c_char = ptr::null_mut();
        let mut color: *mut c_char = ptr::null_mut();
        let mut emotes: *mut c_char = ptr::null_mut();
        let mut reply_display_name: *mut c_char = ptr::null_mut();
        let mut reply_message: *mut c_char = ptr::null_mut();

        if bytes.first() == Some(&b'@')
            && let Some(tags_end) = bytes.iter().position(|byte| *byte == b' ')
        {
            let tags = &bytes[1..tags_end];
            name = extract_irc_tag_bytes(tags, b"display-name");
            color = extract_irc_tag_bytes(tags, b"color");
            emotes = extract_irc_tag_bytes(tags, b"emotes");
            reply_display_name = extract_irc_tag_bytes(tags, b"reply-parent-display-name");
            reply_message = extract_irc_tag_bytes(tags, b"reply-parent-msg-body");
        }

        if name.is_null() {
            name = extract_sender_from_prefix(line);
        }

        let message = dup_bytes(bytes[message_start..].trim_ascii_end());

        Box::into_raw(Box::new(ParsedPrivmsg {
            display_name: name,
            message,
            color,
            emotes,
            reply_display_name,
            reply_message,
        }))
    }
}
