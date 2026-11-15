use super::ffi::{
    G_IO_ERROR_CANCELLED, GCancellable, GDataInputStream, GError, GIOStream, GOutputStream,
    GSocketClient, GSocketConnection, g_cancellable_is_cancelled, g_clear_error,
    g_data_input_stream_new, g_data_input_stream_read_line_utf8, g_error_matches, g_free,
    g_io_error_quark, g_io_stream_get_input_stream, g_io_stream_get_output_stream, g_object_unref,
    g_output_stream_write_all, g_random_int_range, g_socket_client_connect_to_host,
    g_socket_client_new, g_socket_client_set_timeout, g_socket_client_set_tls,
    g_socket_connection_get_socket, g_socket_set_timeout, g_usleep,
};
use super::messages::parse_privmsg;
use super::messages::{channel_message, emit_message, emit_status, prefixed_message};
use super::{ChatWorkerData, twitch_chat_client_unref};
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::ptr;
pub(super) const CHAT_CONNECT_TIMEOUT_SECONDS: c_uint = 15;
pub(super) const CHAT_RECONNECT_DELAY_MS: c_uint = 3000;
pub(super) const CHAT_RECONNECT_POLL_MS: c_uint = 100;

pub(super) unsafe fn error_message(error: *mut GError) -> Vec<u8> {
    unsafe {
        if error.is_null() || (*error).message.is_null() {
            b"unknown error".to_vec()
        } else {
            CStr::from_ptr((*error).message).to_bytes().to_vec()
        }
    }
}

pub(super) unsafe fn write_irc_line(
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

pub(super) unsafe fn wait_before_reconnect(cancel: *mut GCancellable) -> bool {
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

pub(super) unsafe fn cleanup_session_objects(
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

// Join Twitch IRC anonymously and read messages until disconnected.
pub(super) unsafe fn run_chat_session(
    data: *mut ChatWorkerData,
    socket_client: *mut GSocketClient,
) -> bool {
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
        let channel = CStr::from_ptr((*data).channel.as_ptr()).to_bytes();
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
            let line = channel_message(b"Chat verbunden: #", (*data).channel.as_ptr());
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

            if let Some(chat_line) = parse_privmsg(CStr::from_ptr(line).to_bytes()) {
                emit_message((*data).client, (*data).generation, chat_line);
            }
            g_free(line as *mut c_void);
        }

        cleanup_session_objects(data_input, connection, &mut error, false)
    }
}

pub(super) unsafe extern "C" fn chat_worker(user_data: *mut c_void) -> *mut c_void {
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
        twitch_chat_client_unref((*data).client);
        drop(Box::from_raw(data));
        ptr::null_mut()
    }
}
