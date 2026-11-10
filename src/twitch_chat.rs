use crate::glib_strings::dup_bytes;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};

use std::ptr;

const CHAT_CONNECT_TIMEOUT_SECONDS: c_uint = 15;
const CHAT_RECONNECT_DELAY_MS: c_uint = 3000;
const CHAT_RECONNECT_POLL_MS: c_uint = 100;
const CHAT_MAINLOOP_PRIORITY: c_int = 300;
const CHAT_MAX_PENDING_MAIN_LINES: c_int = 128;
const G_IO_ERROR_CANCELLED: c_int = 19;
const G_SOURCE_REMOVE: c_int = 0;
const TWITCH_CHAT_LINE_STATUS: c_int = 0;
const TWITCH_CHAT_LINE_MESSAGE: c_int = 1;

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

#[repr(C)]
pub struct GCancellable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GDataInputStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GError {
    domain: c_uint,
    code: c_int,
    message: *mut c_char,
}

#[repr(C)]
pub struct GInputStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GIOStream {
    _private: [u8; 0],
}

#[repr(C)]
pub union GMutex {
    p: *mut c_void,
    i: [c_uint; 2],
}

#[repr(C)]
pub struct GOutputStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GSocket {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GSocketClient {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GSocketConnection {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GThread {
    _private: [u8; 0],
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;
type GThreadFunc = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
pub type TwitchChatLineFunc = unsafe extern "C" fn(*const TwitchChatLine, *mut c_void);

unsafe extern "C" {
    fn g_ascii_strdown(str: *const c_char, len: isize) -> *mut c_char;
    fn g_atomic_int_add(atomic: *mut c_int, val: c_int) -> c_int;
    fn g_atomic_int_dec_and_test(atomic: *mut c_int) -> c_int;
    fn g_atomic_int_inc(atomic: *mut c_int);
    fn g_cancellable_cancel(cancellable: *mut GCancellable);
    fn g_cancellable_is_cancelled(cancellable: *mut GCancellable) -> c_int;
    fn g_cancellable_new() -> *mut GCancellable;
    fn g_clear_error(error: *mut *mut GError);
    fn g_data_input_stream_new(base_stream: *mut GInputStream) -> *mut GDataInputStream;
    fn g_data_input_stream_read_line_utf8(
        stream: *mut GDataInputStream,
        length: *mut usize,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut c_char;
    fn g_error_matches(error: *mut GError, domain: c_uint, code: c_int) -> c_int;
    fn g_free(mem: *mut c_void);
    fn g_io_error_quark() -> c_uint;
    fn g_io_stream_get_input_stream(stream: *mut GIOStream) -> *mut GInputStream;
    fn g_io_stream_get_output_stream(stream: *mut GIOStream) -> *mut GOutputStream;
    fn g_main_context_invoke_full(
        context: *mut c_void,
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    );
    fn g_mutex_clear(mutex: *mut GMutex);
    fn g_mutex_init(mutex: *mut GMutex);
    fn g_mutex_lock(mutex: *mut GMutex);
    fn g_mutex_unlock(mutex: *mut GMutex);
    fn g_object_ref(object: *mut c_void) -> *mut c_void;
    fn g_object_unref(object: *mut c_void);
    fn g_output_stream_write_all(
        stream: *mut GOutputStream,
        buffer: *const c_void,
        count: usize,
        bytes_written: *mut usize,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> c_int;
    fn g_random_int_range(begin: c_int, end: c_int) -> c_int;
    fn g_socket_client_connect_to_host(
        client: *mut GSocketClient,
        host_and_port: *const c_char,
        default_port: u16,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut GSocketConnection;
    fn g_socket_client_new() -> *mut GSocketClient;
    fn g_socket_client_set_timeout(client: *mut GSocketClient, timeout: c_uint);
    fn g_socket_client_set_tls(client: *mut GSocketClient, tls: c_int);
    fn g_socket_connection_get_socket(connection: *mut GSocketConnection) -> *mut GSocket;
    fn g_socket_set_timeout(socket: *mut GSocket, timeout: c_uint);
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_thread_new(
        name: *const c_char,
        func: Option<GThreadFunc>,
        data: *mut c_void,
    ) -> *mut GThread;
    fn g_thread_unref(thread: *mut GThread);
    fn g_usleep(microseconds: u64);
}

unsafe fn dup_ptr(value: *const c_char) -> *mut c_char {
    unsafe {
        if value.is_null() {
            ptr::null_mut()
        } else {
            g_strdup(value)
        }
    }
}

unsafe fn error_message(error: *mut GError) -> Vec<u8> {
    unsafe {
        if error.is_null() || (*error).message.is_null() {
            b"unknown error".to_vec()
        } else {
            CStr::from_ptr((*error).message).to_bytes().to_vec()
        }
    }
}

unsafe fn prefixed_message(prefix: &[u8], detail: *mut GError) -> Vec<u8> {
    unsafe {
        let message = error_message(detail);
        let mut value = Vec::with_capacity(prefix.len() + message.len() + 1);
        value.extend_from_slice(prefix);
        value.extend_from_slice(&message);
        value.push(0);
        value
    }
}

unsafe fn channel_message(prefix: &[u8], channel: *const c_char) -> Vec<u8> {
    unsafe {
        let channel = CStr::from_ptr(channel).to_bytes();
        let mut value = Vec::with_capacity(prefix.len() + channel.len() + 1);
        value.extend_from_slice(prefix);
        value.extend_from_slice(channel);
        value.push(0);
        value
    }
}

unsafe fn twitch_chat_client_ref(client: *mut TwitchChatClient) -> *mut TwitchChatClient {
    unsafe {
        g_atomic_int_inc(&mut (*client).ref_count);
        client
    }
}

unsafe extern "C" fn chat_line_data_free(data: *mut c_void) {
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

unsafe fn queue_line_on_main(data: *mut ChatLineData) {
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

unsafe extern "C" fn emit_line_on_main(user_data: *mut c_void) -> c_int {
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

unsafe fn emit_status(client: *mut TwitchChatClient, generation: c_uint, message: *const c_char) {
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

unsafe fn emit_message(
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

unsafe fn write_irc_line(
    output: *mut GOutputStream,
    line: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> c_int {
    unsafe {
        let line = CStr::from_ptr(line).to_bytes();
        let mut wire_line = Vec::with_capacity(line.len() + 2);
        wire_line.extend_from_slice(line);
        wire_line.extend_from_slice(b"\r\n");

        let mut written = 0usize;
        g_output_stream_write_all(
            output,
            wire_line.as_ptr() as *const c_void,
            wire_line.len(),
            &mut written,
            cancel,
            error,
        )
    }
}

fn extract_irc_tag_bytes(tags: &[u8], key: &[u8]) -> *mut c_char {
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

unsafe fn extract_sender_from_prefix(line: *const c_char) -> *mut c_char {
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

unsafe fn parsed_privmsg_free(message: *mut ParsedPrivmsg) {
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

unsafe fn parse_privmsg(line: *const c_char) -> *mut ParsedPrivmsg {
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

unsafe fn wait_before_reconnect(cancel: *mut GCancellable) -> bool {
    unsafe {
        let mut elapsed = 0;
        while elapsed < CHAT_RECONNECT_DELAY_MS {
            if g_cancellable_is_cancelled(cancel) != 0 {
                return false;
            }

            g_usleep((CHAT_RECONNECT_POLL_MS * 1000) as u64);
            elapsed += CHAT_RECONNECT_POLL_MS;
        }

        g_cancellable_is_cancelled(cancel) == 0
    }
}

unsafe fn cleanup_session_objects(
    data_input: *mut GDataInputStream,
    connection: *mut GSocketConnection,
    error: *mut *mut GError,
    result: bool,
) -> bool {
    unsafe {
        g_clear_error(error);
        if !data_input.is_null() {
            g_object_unref(data_input as *mut c_void);
        }
        if !connection.is_null() {
            g_object_unref(connection as *mut c_void);
        }
        result
    }
}

unsafe fn run_chat_session(data: *mut ChatWorkerData, socket_client: *mut GSocketClient) -> bool {
    unsafe {
        let mut error: *mut GError = ptr::null_mut();

        let connection = g_socket_client_connect_to_host(
            socket_client,
            c"irc.chat.twitch.tv".as_ptr(),
            6697,
            (*data).cancel,
            &mut error,
        );

        if connection.is_null() {
            if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) != 0 {
                g_clear_error(&mut error);
                return false;
            }

            if g_cancellable_is_cancelled((*data).cancel) == 0 {
                let line = prefixed_message(b"Chat-Verbindung fehlgeschlagen: ", error);
                emit_status(
                    (*data).client,
                    (*data).generation,
                    line.as_ptr() as *const c_char,
                );
            }
            g_clear_error(&mut error);
            return true;
        }

        let socket = g_socket_connection_get_socket(connection);
        if !socket.is_null() {
            g_socket_set_timeout(socket, 0);
        }

        let output = g_io_stream_get_output_stream(connection as *mut GIOStream);
        let input = g_io_stream_get_input_stream(connection as *mut GIOStream);
        let data_input = g_data_input_stream_new(input);

        let nick = format!("justinfan{}\0", g_random_int_range(10000, 999999));
        let nick_line = format!("NICK {}\0", &nick[..nick.len() - 1]);
        let channel = CStr::from_ptr((*data).channel).to_bytes();
        let mut join_line = Vec::with_capacity(channel.len() + 8);
        join_line.extend_from_slice(b"JOIN #");
        join_line.extend_from_slice(channel);
        join_line.push(0);

        if write_irc_line(
            output,
            c"CAP REQ :twitch.tv/tags twitch.tv/commands".as_ptr(),
            (*data).cancel,
            &mut error,
        ) == 0
            || write_irc_line(
                output,
                nick_line.as_ptr() as *const c_char,
                (*data).cancel,
                &mut error,
            ) == 0
            || write_irc_line(
                output,
                join_line.as_ptr() as *const c_char,
                (*data).cancel,
                &mut error,
            ) == 0
        {
            let result = if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) != 0 {
                false
            } else {
                if g_cancellable_is_cancelled((*data).cancel) == 0 {
                    let line = prefixed_message(b"Chat-Login fehlgeschlagen: ", error);
                    emit_status(
                        (*data).client,
                        (*data).generation,
                        line.as_ptr() as *const c_char,
                    );
                }
                true
            };
            return cleanup_session_objects(data_input, connection, &mut error, result);
        }

        {
            let line = channel_message(b"Chat verbunden: #", (*data).channel);
            emit_status(
                (*data).client,
                (*data).generation,
                line.as_ptr() as *const c_char,
            );
        }

        while g_cancellable_is_cancelled((*data).cancel) == 0 {
            let mut length = 0usize;
            g_clear_error(&mut error);
            let line = g_data_input_stream_read_line_utf8(
                data_input,
                &mut length,
                (*data).cancel,
                &mut error,
            );

            if line.is_null() {
                let result = if !error.is_null()
                    && g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) != 0
                {
                    false
                } else {
                    if !error.is_null() && g_cancellable_is_cancelled((*data).cancel) == 0 {
                        let message = prefixed_message(b"Chat getrennt: ", error);
                        emit_status(
                            (*data).client,
                            (*data).generation,
                            message.as_ptr() as *const c_char,
                        );
                    } else if g_cancellable_is_cancelled((*data).cancel) == 0 {
                        emit_status(
                            (*data).client,
                            (*data).generation,
                            c"Chat getrennt".as_ptr(),
                        );
                    }
                    true
                };
                return cleanup_session_objects(data_input, connection, &mut error, result);
            }

            let line_bytes = CStr::from_ptr(line).to_bytes();
            if line_bytes.starts_with(b"PING ") {
                let mut pong = Vec::with_capacity(line_bytes.len() + 1);
                pong.extend_from_slice(b"PONG ");
                pong.extend_from_slice(&line_bytes[5..]);
                pong.push(0);
                g_clear_error(&mut error);
                if write_irc_line(
                    output,
                    pong.as_ptr() as *const c_char,
                    (*data).cancel,
                    &mut error,
                ) == 0
                {
                    g_free(line as *mut c_void);
                    let result =
                        if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) != 0 {
                            false
                        } else {
                            if g_cancellable_is_cancelled((*data).cancel) == 0 {
                                let message = prefixed_message(b"Chat getrennt: ", error);
                                emit_status(
                                    (*data).client,
                                    (*data).generation,
                                    message.as_ptr() as *const c_char,
                                );
                            }
                            true
                        };
                    return cleanup_session_objects(data_input, connection, &mut error, result);
                }
                g_free(line as *mut c_void);
                continue;
            }

            let chat_line = parse_privmsg(line);
            if !chat_line.is_null() {
                emit_message((*data).client, (*data).generation, chat_line);
                parsed_privmsg_free(chat_line);
            }
            g_free(line as *mut c_void);
        }

        cleanup_session_objects(data_input, connection, &mut error, false)
    }
}

unsafe extern "C" fn chat_worker(user_data: *mut c_void) -> *mut c_void {
    unsafe {
        let data = user_data as *mut ChatWorkerData;
        let socket_client = g_socket_client_new();

        g_socket_client_set_tls(socket_client, 1);
        g_socket_client_set_timeout(socket_client, CHAT_CONNECT_TIMEOUT_SECONDS);

        while g_cancellable_is_cancelled((*data).cancel) == 0 {
            let reconnect = run_chat_session(data, socket_client);

            if !reconnect || g_cancellable_is_cancelled((*data).cancel) != 0 {
                break;
            }

            emit_status(
                (*data).client,
                (*data).generation,
                c"Chat verbindet in 3 Sekunden neu ...".as_ptr(),
            );
            if !wait_before_reconnect((*data).cancel) {
                break;
            }
        }

        g_object_unref(socket_client as *mut c_void);
        if !(*data).cancel.is_null() {
            g_object_unref((*data).cancel as *mut c_void);
        }
        g_free((*data).channel as *mut c_void);
        twitch_chat_client_unref((*data).client);
        drop(Box::from_raw(data));
        ptr::null_mut()
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
            mutex: GMutex { p: ptr::null_mut() },
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

#[cfg(test)]
mod tests {
    use super as twitch_chat;
    use std::ffi::{CStr, CString, c_char, c_void};

    use super::g_free;

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
        twitch_chat::extract_irc_tag_bytes(tags.as_bytes(), key.as_bytes())
    }

    unsafe fn extract_sender_from_prefix(line: &str) -> *mut c_char {
        unsafe { twitch_chat::extract_sender_from_prefix(cstring(line).as_ptr()) }
    }

    unsafe fn parse_privmsg(line: &str) -> *mut twitch_chat::ParsedPrivmsg {
        unsafe { twitch_chat::parse_privmsg(cstring(line).as_ptr()) }
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

            twitch_chat::parsed_privmsg_free(message);
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

            twitch_chat::parsed_privmsg_free(message);
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
