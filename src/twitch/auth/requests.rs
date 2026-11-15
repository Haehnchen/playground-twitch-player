use super::ffi::{
    G_IO_ERROR_FAILED, G_IO_ERROR_TIMED_OUT, G_USEC_PER_SEC, GTask, g_bytes_get_data,
    g_bytes_new_static, g_bytes_unref, g_cancellable_set_error_if_cancelled, g_free,
    g_get_monotonic_time, g_io_error_quark, g_object_set, g_object_unref, g_set_error, g_strcmp0,
    g_strndup, g_task_return_error, g_task_return_pointer, g_uri_escape_string, g_usleep,
    soup_message_get_request_headers, soup_message_get_status, soup_message_headers_append,
    soup_message_new, soup_message_set_request_body_from_bytes, soup_session_new,
    soup_session_send_and_read,
};
use super::parser::{parse_auth_error_message, parse_device_code_response, parse_token_response};
use super::{
    AuthResponse, DeviceCodeData, PollTokenData, TwitchAuthDeviceCode, TwitchAuthToken,
    twitch_auth_device_code_free, twitch_auth_token_free,
};
use crate::twitch::stream_info::{GCancellable, GError};
use std::ffi::{CStr, c_char, c_int, c_uint, c_ulong, c_void};
use std::ptr;

pub(super) const TWITCH_DEVICE_URI: &[u8] = b"https://id.twitch.tv/oauth2/device\0";

pub(super) const TWITCH_TOKEN_URI: &[u8] = b"https://id.twitch.tv/oauth2/token\0";

pub(super) const TWITCH_FOLLOWS_SCOPE: &[u8] = b"user:read:follows\0";

pub(super) const TWITCH_DEVICE_GRANT: &[u8] = b"urn:ietf:params:oauth:grant-type:device_code\0";

pub(super) const TWITCH_REFRESH_GRANT: &[u8] = b"refresh_token\0";

pub(super) unsafe fn append_form_pair(
    form: &mut Vec<u8>,
    name: *const c_char,
    value: *const c_char,
) {
    unsafe {
        if !form.is_empty() {
            form.push(b'&');
        }

        let escaped_name = g_uri_escape_string(name, ptr::null(), 1);
        let empty = c"".as_ptr();
        let escaped_value =
            g_uri_escape_string(if value.is_null() { empty } else { value }, ptr::null(), 1);

        form.extend_from_slice(CStr::from_ptr(escaped_name).to_bytes());
        form.push(b'=');
        form.extend_from_slice(CStr::from_ptr(escaped_value).to_bytes());

        g_free(escaped_name as *mut c_void);
        g_free(escaped_value as *mut c_void);
    }
}

pub(super) unsafe fn post_auth_form(
    uri: *const c_char,
    body: &[u8],
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut AuthResponse {
    unsafe {
        let session = soup_session_new();
        let message = soup_message_new(c"POST".as_ptr(), uri);
        let body_bytes = g_bytes_new_static(body.as_ptr() as *const c_void, body.len());

        g_object_set(
            session as *mut c_void,
            c"timeout".as_ptr(),
            15,
            ptr::null::<c_char>(),
        );
        let request_headers = soup_message_get_request_headers(message);
        soup_message_headers_append(
            request_headers,
            c"Accept".as_ptr(),
            c"application/json".as_ptr(),
        );
        soup_message_set_request_body_from_bytes(
            message,
            c"application/x-www-form-urlencoded".as_ptr(),
            body_bytes,
        );

        let response_bytes = soup_session_send_and_read(session, message, cancel, error);
        g_bytes_unref(body_bytes);
        if response_bytes.is_null() {
            g_object_unref(message as *mut c_void);
            g_object_unref(session as *mut c_void);
            return ptr::null_mut();
        }

        let mut response_size = 0usize;
        let response_data = g_bytes_get_data(response_bytes, &mut response_size) as *const c_char;
        let response = Box::new(AuthResponse {
            status: soup_message_get_status(message),
            body: g_strndup(response_data, response_size),
        });

        g_bytes_unref(response_bytes);
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        Box::into_raw(response)
    }
}

// Wait between token requests while checking for cancellation.
pub(super) unsafe fn sleep_poll_interval(
    interval: c_uint,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> c_int {
    unsafe {
        let mut remaining_ms = interval.max(1) * 1000;
        while remaining_ms > 0 {
            if g_cancellable_set_error_if_cancelled(cancel, error) != 0 {
                return 0;
            }

            let chunk_ms = remaining_ms.min(100);
            g_usleep((chunk_ms * 1000) as c_ulong);
            remaining_ms -= chunk_ms;
        }

        1
    }
}

pub(super) unsafe fn request_device_code(
    data: *mut DeviceCodeData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchAuthDeviceCode {
    unsafe {
        let mut form = Vec::new();
        append_form_pair(&mut form, c"client_id".as_ptr(), (*data).client_id);
        append_form_pair(
            &mut form,
            c"scopes".as_ptr(),
            TWITCH_FOLLOWS_SCOPE.as_ptr() as *const c_char,
        );

        let response = post_auth_form(
            TWITCH_DEVICE_URI.as_ptr() as *const c_char,
            &form,
            cancel,
            error,
        );
        if response.is_null() {
            return ptr::null_mut();
        }

        if (*response).status < 200 || (*response).status >= 300 {
            let message = parse_auth_error_message((*response).body);
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch auth returned HTTP %u%s%s".as_ptr(),
                (*response).status,
                if *message != 0 {
                    c": ".as_ptr()
                } else {
                    c"".as_ptr()
                },
                message,
            );
            g_free(message as *mut c_void);
            auth_response_free(response);
            return ptr::null_mut();
        }

        let code = parse_device_code_response((*response).body, error);
        auth_response_free(response);
        code
    }
}

// Poll for authorization until the device code expires or polling is cancelled.
pub(super) unsafe fn poll_device_token(
    data: *mut PollTokenData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    unsafe {
        let mut interval = if (*data).interval > 0 {
            (*data).interval
        } else {
            5
        };
        let deadline_us =
            g_get_monotonic_time() + (*data).expires_in.max(1) as i64 * G_USEC_PER_SEC;

        while g_get_monotonic_time() < deadline_us {
            if g_cancellable_set_error_if_cancelled(cancel, error) != 0 {
                return ptr::null_mut();
            }

            let mut form = Vec::new();
            append_form_pair(&mut form, c"client_id".as_ptr(), (*data).client_id);
            append_form_pair(
                &mut form,
                c"scope".as_ptr(),
                TWITCH_FOLLOWS_SCOPE.as_ptr() as *const c_char,
            );
            append_form_pair(&mut form, c"device_code".as_ptr(), (*data).device_code);
            append_form_pair(
                &mut form,
                c"grant_type".as_ptr(),
                TWITCH_DEVICE_GRANT.as_ptr() as *const c_char,
            );

            let response = post_auth_form(
                TWITCH_TOKEN_URI.as_ptr() as *const c_char,
                &form,
                cancel,
                error,
            );
            if response.is_null() {
                return ptr::null_mut();
            }

            if (*response).status >= 200 && (*response).status < 300 {
                let token = parse_token_response((*response).body, error);
                auth_response_free(response);
                return token;
            }

            let message = parse_auth_error_message((*response).body);
            auth_response_free(response);
            if g_strcmp0(message, c"authorization_pending".as_ptr()) == 0 {
                g_free(message as *mut c_void);
                if sleep_poll_interval(interval, cancel, error) == 0 {
                    return ptr::null_mut();
                }
                continue;
            }
            if g_strcmp0(message, c"slow_down".as_ptr()) == 0 {
                g_free(message as *mut c_void);
                interval += 5;
                if sleep_poll_interval(interval, cancel, error) == 0 {
                    return ptr::null_mut();
                }
                continue;
            }

            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch authorization failed%s%s".as_ptr(),
                if *message != 0 {
                    c": ".as_ptr()
                } else {
                    c"".as_ptr()
                },
                message,
            );
            g_free(message as *mut c_void);
            return ptr::null_mut();
        }

        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_TIMED_OUT,
            c"Twitch authorization timed out".as_ptr(),
        );
        ptr::null_mut()
    }
}

/// # Safety
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
/// - `cancel`: null or live `GCancellable`.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Result owned; release with matching token/array free function.
pub unsafe fn twitch_auth_refresh_token(
    client_id: *const c_char,
    refresh_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    unsafe {
        if client_id.is_null() || *client_id == 0 || refresh_token.is_null() || *refresh_token == 0
        {
            return ptr::null_mut();
        }

        let mut form = Vec::new();
        append_form_pair(&mut form, c"client_id".as_ptr(), client_id);
        append_form_pair(
            &mut form,
            c"grant_type".as_ptr(),
            TWITCH_REFRESH_GRANT.as_ptr() as *const c_char,
        );
        append_form_pair(&mut form, c"refresh_token".as_ptr(), refresh_token);

        let response = post_auth_form(
            TWITCH_TOKEN_URI.as_ptr() as *const c_char,
            &form,
            cancel,
            error,
        );
        if response.is_null() {
            return ptr::null_mut();
        }

        if (*response).status < 200 || (*response).status >= 300 {
            let message = parse_auth_error_message((*response).body);
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch token refresh failed with HTTP %u%s%s".as_ptr(),
                (*response).status,
                if *message != 0 {
                    c": ".as_ptr()
                } else {
                    c"".as_ptr()
                },
                message,
            );
            g_free(message as *mut c_void);
            auth_response_free(response);
            return ptr::null_mut();
        }

        let token = parse_token_response((*response).body, error);
        auth_response_free(response);
        if token.is_null() {
            return ptr::null_mut();
        }
        if (*token).refresh_token.is_null() || *(*token).refresh_token == 0 {
            twitch_auth_token_free(token);
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch did not return a refresh token".as_ptr(),
            );
            return ptr::null_mut();
        }

        token
    }
}

pub(super) unsafe extern "C" fn device_code_data_free(data: *mut c_void) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data as *mut DeviceCodeData);
        g_free(data.client_id as *mut c_void);
    }
}

pub(super) unsafe extern "C" fn poll_token_data_free(data: *mut c_void) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data as *mut PollTokenData);
        g_free(data.client_id as *mut c_void);
        g_free(data.device_code as *mut c_void);
    }
}

pub(super) unsafe fn auth_response_free(response: *mut AuthResponse) {
    unsafe {
        if response.is_null() {
            return;
        }

        let response = Box::from_raw(response);
        g_free(response.body as *mut c_void);
    }
}

pub(super) unsafe extern "C" fn request_device_code_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
        let data = task_data as *mut DeviceCodeData;
        let mut error: *mut GError = ptr::null_mut();
        let code = request_device_code(data, cancel, &mut error);

        if !error.is_null() {
            g_task_return_error(task, error);
            return;
        }

        g_task_return_pointer(
            task,
            code as *mut c_void,
            Some(twitch_auth_device_code_free_as_destroy),
        );
    }
}

pub(super) unsafe extern "C" fn poll_device_token_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
        let data = task_data as *mut PollTokenData;
        let mut error: *mut GError = ptr::null_mut();
        let token = poll_device_token(data, cancel, &mut error);

        if !error.is_null() {
            g_task_return_error(task, error);
            return;
        }

        g_task_return_pointer(
            task,
            token as *mut c_void,
            Some(twitch_auth_token_free_as_destroy),
        );
    }
}

pub(super) unsafe extern "C" fn twitch_auth_device_code_free_as_destroy(data: *mut c_void) {
    unsafe {
        twitch_auth_device_code_free(data as *mut TwitchAuthDeviceCode);
    }
}

pub(super) unsafe extern "C" fn twitch_auth_token_free_as_destroy(data: *mut c_void) {
    unsafe {
        twitch_auth_token_free(data as *mut TwitchAuthToken);
    }
}
