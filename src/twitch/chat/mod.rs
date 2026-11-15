mod ffi;
mod messages;
mod worker;

pub use self::ffi::{
    GCancellable, GDataInputStream, GError, GIOStream, GInputStream, GOutputStream, GSocket,
    GSocketClient, GSocketConnection, GThread,
};

use self::ffi::{
    g_cancellable_cancel, g_cancellable_new, g_object_ref, g_object_unref, g_thread_new,
    g_thread_unref,
};
use self::worker::chat_worker;
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

pub struct TwitchChatClient {
    state: std::sync::Mutex<ChatClientState>,
    ref_count: std::sync::atomic::AtomicUsize,
    pending_lines: std::sync::atomic::AtomicUsize,
}

struct ChatClientState {
    line_func: Option<TwitchChatLineFunc>,
    user_data: *mut c_void,
    thread: *mut GThread,
    cancel: *mut GCancellable,
    generation: c_uint,
    closed: bool,
}

pub struct ChatWorkerData {
    client: *mut TwitchChatClient,
    channel: CString,
    generation: c_uint,
    cancel: *mut GCancellable,
}

pub struct ChatLineData {
    client: *mut TwitchChatClient,
    generation: c_uint,
    line: TwitchChatLine,

    counted_pending: bool,
}

#[derive(Default)]
pub struct TwitchChatLine {
    pub kind: c_int,
    pub display_name: CString,
    pub message: CString,
    pub color: CString,
    pub emotes: CString,
    pub reply_display_name: CString,
    pub reply_message: CString,
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

type GThreadFunc = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

pub type TwitchChatLineFunc = unsafe extern "C" fn(*const TwitchChatLine, *mut c_void);

unsafe fn twitch_chat_client_ref(client: *mut TwitchChatClient) -> *mut TwitchChatClient {
    unsafe {
        (*client).ref_count.fetch_add(1, Ordering::Relaxed);
        client
    }
}

unsafe fn twitch_chat_client_cancel_current(client: *mut TwitchChatClient) {
    unsafe {
        let (thread, cancel) = {
            let mut state = (*client).state.lock().expect("chat state poisoned");
            (
                std::mem::replace(&mut state.thread, ptr::null_mut()),
                std::mem::replace(&mut state.cancel, ptr::null_mut()),
            )
        };
        if !cancel.is_null() {
            g_cancellable_cancel(cancel);
            g_object_unref(cancel.cast());
        }
        if !thread.is_null() {
            g_thread_unref(thread);
        }
    }
}

unsafe fn twitch_chat_client_unref(client: *mut TwitchChatClient) {
    unsafe {
        if !client.is_null() && (*client).ref_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            twitch_chat_client_cancel_current(client);
            drop(Box::from_raw(client));
        }
    }
}

/// # Safety
/// Callback data stays live until client free; callbacks run on the default main context.
/// The callback uses TwitchChatLineFunc's ABI and must not unwind.
pub unsafe fn twitch_chat_client_new(
    line_func: Option<TwitchChatLineFunc>,
    user_data: *mut c_void,
) -> *mut TwitchChatClient {
    Box::into_raw(Box::new(TwitchChatClient {
        state: std::sync::Mutex::new(ChatClientState {
            line_func,
            user_data,
            thread: ptr::null_mut(),
            cancel: ptr::null_mut(),
            generation: 0,
            closed: false,
        }),
        ref_count: std::sync::atomic::AtomicUsize::new(1),
        pending_lines: std::sync::atomic::AtomicUsize::new(0),
    }))
}

/// # Safety
/// Live client on the default main context; channel is readable and NUL-terminated.
pub unsafe fn twitch_chat_client_start(client: *mut TwitchChatClient, channel: *const c_char) {
    unsafe {
        if client.is_null() || channel.is_null() || *channel == 0 {
            return;
        }
        let channel =
            CString::new(CStr::from_ptr(channel).to_bytes().to_ascii_lowercase()).unwrap();
        twitch_chat_client_cancel_current(client);
        let mut state = (*client).state.lock().expect("chat state poisoned");
        if state.closed {
            return;
        }
        state.generation = state.generation.wrapping_add(1);
        state.cancel = g_cancellable_new();
        let data = ChatWorkerData {
            client: twitch_chat_client_ref(client),
            channel,
            generation: state.generation,
            cancel: g_object_ref(state.cancel.cast()).cast(),
        };
        state.thread = g_thread_new(
            c"twitch-chat".as_ptr(),
            Some(chat_worker),
            Box::into_raw(Box::new(data)).cast(),
        );
    }
}

/// # Safety
/// Null or live client; serialize calls on the default main context.
pub unsafe fn twitch_chat_client_stop(client: *mut TwitchChatClient) {
    unsafe {
        if client.is_null() {
            return;
        }
        {
            let mut state = (*client).state.lock().expect("chat state poisoned");
            state.generation = state.generation.wrapping_add(1);
        }
        twitch_chat_client_cancel_current(client);
    }
}

/// # Safety
/// Null or owned client on the default main context; release the owner once.
pub unsafe fn twitch_chat_client_free(client: *mut TwitchChatClient) {
    unsafe {
        if client.is_null() {
            return;
        }
        {
            let mut state = (*client).state.lock().expect("chat state poisoned");
            state.closed = true;
            state.line_func = None;
            state.user_data = ptr::null_mut();
            state.generation = state.generation.wrapping_add(1);
        }
        twitch_chat_client_cancel_current(client);
        twitch_chat_client_unref(client);
    }
}

#[cfg(test)]
mod tests {
    use super::messages::{extract_irc_tag_bytes, extract_sender_from_prefix, parse_privmsg};
    use super::*;

    #[test]
    fn queued_messages_ignore_old_generations_and_closed_clients() {
        unsafe extern "C" fn received(_line: *const TwitchChatLine, data: *mut c_void) {
            unsafe {
                *data.cast::<usize>() += 1;
            }
        }
        unsafe {
            let mut count = 0_usize;
            let client = twitch_chat_client_new(Some(received), ptr::from_mut(&mut count).cast());
            let data = Box::into_raw(Box::new(ChatLineData {
                client: twitch_chat_client_ref(client),
                generation: 0,
                line: TwitchChatLine::default(),
                counted_pending: false,
            }));
            messages::emit_line_on_main(data.cast());
            assert_eq!(count, 1);
            twitch_chat_client_stop(client);
            messages::emit_line_on_main(data.cast());
            assert_eq!(count, 1);
            (*data).generation = (*client).state.lock().unwrap().generation;
            twitch_chat_client_free(client);
            messages::emit_line_on_main(data.cast());
            assert_eq!(count, 1);
            messages::chat_line_data_free(data.cast());
        }
    }

    #[test]
    fn irc_tags_decode_escapes_and_match_exact_keys() {
        let tags = br"display-name=Alice\sBob;reply-parent-msg-body=hello\sworld\:\\line\r\n;room-id=123;id=456";
        assert_eq!(
            extract_irc_tag_bytes(tags, b"display-name").as_bytes(),
            b"Alice Bob"
        );
        assert_eq!(
            extract_irc_tag_bytes(tags, b"reply-parent-msg-body").as_bytes(),
            b"hello world;\\line\r\n"
        );
        assert_eq!(extract_irc_tag_bytes(tags, b"id").as_bytes(), b"456");
        assert!(extract_irc_tag_bytes(b"display-name=;", b"display-name").is_empty());
    }

    #[test]
    fn irc_messages_keep_tags_and_reply_metadata() {
        let line = br"@display-name=Alice\sBob;color=#123456;emotes=25:0-4;reply-parent-display-name=Carol;reply-parent-msg-body=previous\smessage :alice!alice@example.test PRIVMSG #channel :Kappa hello";
        let message = parse_privmsg(line).unwrap();
        assert_eq!(message.display_name.as_bytes(), b"Alice Bob");
        assert_eq!(message.message.as_bytes(), b"Kappa hello");
        assert_eq!(message.color.as_bytes(), b"#123456");
        assert_eq!(message.emotes.as_bytes(), b"25:0-4");
        assert_eq!(message.reply_display_name.as_bytes(), b"Carol");
        assert_eq!(message.reply_message.as_bytes(), b"previous message");
    }

    #[test]
    fn irc_messages_use_sender_when_tags_are_missing() {
        let message =
            parse_privmsg(b":fallback!user@example.test PRIVMSG #channel :hello\r").unwrap();
        assert_eq!(message.display_name.as_bytes(), b"fallback");
        assert_eq!(message.message.as_bytes(), b"hello");
        assert!(message.color.is_empty());
        assert_eq!(
            extract_sender_from_prefix(b"PING :tmi.twitch.tv").as_bytes(),
            b"chat"
        );
    }

    #[test]
    fn irc_parser_ignores_other_commands() {
        for line in [
            b"PING :tmi.twitch.tv".as_slice(),
            b":server NOTICE #channel :hello",
        ] {
            assert!(parse_privmsg(line).is_none());
        }
    }
}
