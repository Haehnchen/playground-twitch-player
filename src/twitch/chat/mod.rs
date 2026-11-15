mod ffi;
mod messages;
#[cfg(test)]
mod tests {
    use super::ParsedPrivmsg;
    use super::ffi::g_free;
    use super::messages::{extract_irc_tag_bytes, parsed_privmsg_free};
    use std::ffi::{CStr, CString, c_char, c_void};

    fn cstring(value: &str) -> CString {
        CString::new(value).unwrap()
    }

    unsafe fn assert_str_eq(actual: *const c_char, expected: &str) {
        unsafe {
            assert!(!actual.is_null());
            assert_eq!(CStr::from_ptr(actual).to_str().unwrap(), expected);
        }
    }

    fn extract_irc_tag(tags: &str, key: &str) -> *mut c_char {
        extract_irc_tag_bytes(tags.as_bytes(), key.as_bytes())
    }

    unsafe fn extract_sender_from_prefix(line: &str) -> *mut c_char {
        unsafe { super::messages::extract_sender_from_prefix(cstring(line).as_ptr()) }
    }

    unsafe fn parse_privmsg(line: &str) -> *mut ParsedPrivmsg {
        unsafe { super::messages::parse_privmsg(cstring(line).as_ptr()) }
    }

    #[test]
    fn test_extract_irc_tag_returns_decoded_value() {
        unsafe {
            let display_name = extract_irc_tag(
                "display-name=Alice\\sBob;color=#123456;emotes=25:0-4",
                "display-name",
            );
            let reply = extract_irc_tag(
                "reply-parent-msg-body=hello\\sworld\\:\\\\line\\r\\n;other=x",
                "reply-parent-msg-body",
            );

            assert_str_eq(display_name, "Alice Bob");
            assert_str_eq(reply, "hello world;\\line\r\n");
            g_free(display_name as *mut c_void);
            g_free(reply as *mut c_void);
        }
    }

    #[test]
    fn test_extract_irc_tag_matches_exact_key() {
        unsafe {
            let id = extract_irc_tag("room-id=123;id=456", "id");
            let missing = extract_irc_tag("display-name=;", "display-name");

            assert_str_eq(id, "456");
            assert!(missing.is_null());
            g_free(id as *mut c_void);
        }
    }

    #[test]
    fn test_extract_sender_from_prefix() {
        unsafe {
            let sender =
                extract_sender_from_prefix(":alice!alice@example.test PRIVMSG #chan :hello");
            let fallback = extract_sender_from_prefix("PING :tmi.twitch.tv");

            assert_str_eq(sender, "alice");
            assert_str_eq(fallback, "chat");
            g_free(sender as *mut c_void);
            g_free(fallback as *mut c_void);
        }
    }

    #[test]
    fn test_parse_privmsg_with_tags() {
        unsafe {
            let message = parse_privmsg(
                "@display-name=Alice\\sBob;color=#123456;emotes=25:0-4;\
             reply-parent-display-name=Carol;\
             reply-parent-msg-body=previous\\smessage \
             :alice!alice@example.test PRIVMSG #channel :Kappa hello\r",
            );

            assert!(!message.is_null());
            assert_str_eq((*message).display_name, "Alice Bob");
            assert_str_eq((*message).message, "Kappa hello");
            assert_str_eq((*message).color, "#123456");
            assert_str_eq((*message).emotes, "25:0-4");
            assert_str_eq((*message).reply_display_name, "Carol");
            assert_str_eq((*message).reply_message, "previous message");

            parsed_privmsg_free(message);
        }
    }

    #[test]
    fn test_parse_privmsg_falls_back_to_sender() {
        unsafe {
            let message = parse_privmsg(":fallback!user@example.test PRIVMSG #channel :hello");

            assert!(!message.is_null());
            assert_str_eq((*message).display_name, "fallback");
            assert_str_eq((*message).message, "hello");
            assert!((*message).color.is_null());

            parsed_privmsg_free(message);
        }
    }

    #[test]
    fn test_parse_privmsg_rejects_non_messages() {
        unsafe {
            assert!(parse_privmsg("PING :tmi.twitch.tv").is_null());
            assert!(parse_privmsg(":server NOTICE #channel :hello").is_null());
        }
    }
}
mod worker;

pub use self::ffi::{
    GCancellable, GDataInputStream, GError, GIOStream, GInputStream, GMutex, GOutputStream,
    GSocket, GSocketClient, GSocketConnection, GThread,
};

use self::ffi::{
    g_ascii_strdown, g_atomic_int_dec_and_test, g_atomic_int_inc, g_cancellable_cancel,
    g_cancellable_new, g_free, g_mutex_clear, g_mutex_init, g_mutex_lock, g_mutex_unlock,
    g_object_ref, g_object_unref, g_thread_new, g_thread_unref,
};
use self::worker::chat_worker;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

pub struct TwitchChatClient {
    line_func: Option<TwitchChatLineFunc>,
    user_data: *mut c_void,

    thread: *mut GThread,
    cancel: *mut GCancellable,
    mutex: GMutex,

    ref_count: c_int,
    pending_lines: c_int,
    generation: c_uint,
    closed: c_int,
}

pub struct ChatWorkerData {
    client: *mut TwitchChatClient,
    channel: *mut c_char,
    generation: c_uint,
    cancel: *mut GCancellable,
}

pub struct ChatLineData {
    client: *mut TwitchChatClient,
    generation: c_uint,
    line: TwitchChatLine,

    display_name: *mut c_char,
    message: *mut c_char,
    color: *mut c_char,
    emotes: *mut c_char,
    reply_display_name: *mut c_char,
    reply_message: *mut c_char,

    counted_pending: c_int,
}

pub struct TwitchChatLine {
    pub kind: c_int,
    pub display_name: *const c_char,
    pub message: *const c_char,
    pub color: *const c_char,
    pub emotes: *const c_char,
    pub reply_display_name: *const c_char,
    pub reply_message: *const c_char,
}

pub struct ParsedPrivmsg {
    pub display_name: *mut c_char,
    pub message: *mut c_char,
    pub color: *mut c_char,
    pub emotes: *mut c_char,
    pub reply_display_name: *mut c_char,
    pub reply_message: *mut c_char,
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

type GThreadFunc = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

pub type TwitchChatLineFunc = unsafe extern "C" fn(*const TwitchChatLine, *mut c_void);

unsafe fn twitch_chat_client_ref(client: *mut TwitchChatClient) -> *mut TwitchChatClient {
    unsafe {
        g_atomic_int_inc(&mut (*client).ref_count);
        client
    }
}

unsafe fn twitch_chat_client_cancel_current(client: *mut TwitchChatClient) {
    unsafe {
        let mut thread: *mut GThread = ptr::null_mut();
        let mut cancel: *mut GCancellable = ptr::null_mut();

        g_mutex_lock(&mut (*client).mutex);
        if !(*client).thread.is_null() {
            thread = (*client).thread;
            (*client).thread = ptr::null_mut();
        }
        if !(*client).cancel.is_null() {
            cancel = (*client).cancel;
            (*client).cancel = ptr::null_mut();
        }
        g_mutex_unlock(&mut (*client).mutex);

        if !cancel.is_null() {
            g_cancellable_cancel(cancel);
            g_object_unref(cancel as *mut c_void);
        }
        if !thread.is_null() {
            g_thread_unref(thread);
        }
    }
}

unsafe fn twitch_chat_client_unref(client: *mut TwitchChatClient) {
    unsafe {
        if client.is_null() || g_atomic_int_dec_and_test(&mut (*client).ref_count) == 0 {
            return;
        }

        if !(*client).thread.is_null() {
            g_thread_unref((*client).thread);
        }
        if !(*client).cancel.is_null() {
            g_object_unref((*client).cancel as *mut c_void);
        }
        g_mutex_clear(&mut (*client).mutex);
        drop(Box::from_raw(client));
    }
}

/// # Safety
/// - Callback: declared C ABI, no unwind; data valid until client free.
/// - Line arguments borrowed only during callback.
/// - Client operations and callbacks serialized on default GLib main context.
pub unsafe fn twitch_chat_client_new(
    line_func: Option<TwitchChatLineFunc>,
    user_data: *mut c_void,
) -> *mut TwitchChatClient {
    unsafe {
        let client = Box::into_raw(Box::new(TwitchChatClient {
            line_func,
            user_data,

            thread: ptr::null_mut(),
            cancel: ptr::null_mut(),
            mutex: GMutex {
                p: ptr::null_mut(),
            },

            ref_count: 1,
            pending_lines: 0,
            generation: 0,
            closed: 0,
        }));
        g_mutex_init(&mut (*client).mutex);
        client
    }
}

/// # Safety
/// - `client`: null or live instance; default GLib main context, serialized callbacks.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn twitch_chat_client_start(client: *mut TwitchChatClient, channel: *const c_char) {
    unsafe {
        if client.is_null() || channel.is_null() || *channel == 0 {
            return;
        }

        twitch_chat_client_cancel_current(client);

        let data = Box::into_raw(Box::new(ChatWorkerData {
            client: twitch_chat_client_ref(client),
            channel: g_ascii_strdown(channel, -1),
            generation: 0,
            cancel: ptr::null_mut(),
        }));

        g_mutex_lock(&mut (*client).mutex);
        if (*client).closed != 0 {
            g_mutex_unlock(&mut (*client).mutex);
            g_free((*data).channel as *mut c_void);
            twitch_chat_client_unref((*data).client);
            drop(Box::from_raw(data));
            return;
        }

        (*client).generation += 1;
        (*client).cancel = g_cancellable_new();
        (*data).generation = (*client).generation;
        (*data).cancel = g_object_ref((*client).cancel as *mut c_void) as *mut GCancellable;
        (*client).thread = g_thread_new(
            c"twitch-chat".as_ptr(),
            Some(chat_worker),
            data as *mut c_void,
        );
        g_mutex_unlock(&mut (*client).mutex);
    }
}

/// # Safety
/// - `client`: null or live instance; default GLib main context, serialized callbacks.
pub unsafe fn twitch_chat_client_stop(client: *mut TwitchChatClient) {
    unsafe {
        if client.is_null() {
            return;
        }

        g_mutex_lock(&mut (*client).mutex);
        (*client).generation += 1;
        g_mutex_unlock(&mut (*client).mutex);

        twitch_chat_client_cancel_current(client);
    }
}

/// # Safety
/// - `client`: null or live instance; default GLib main context, serialized callbacks.
/// - Release owner once; no subsequent pointer access.
pub unsafe fn twitch_chat_client_free(client: *mut TwitchChatClient) {
    unsafe {
        if client.is_null() {
            return;
        }

        g_mutex_lock(&mut (*client).mutex);
        (*client).closed = 1;
        (*client).line_func = None;
        (*client).user_data = ptr::null_mut();
        (*client).generation += 1;
        g_mutex_unlock(&mut (*client).mutex);

        twitch_chat_client_cancel_current(client);
        twitch_chat_client_unref(client);
    }
}
