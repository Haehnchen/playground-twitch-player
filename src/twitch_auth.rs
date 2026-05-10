use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void, CStr};
use std::ptr;

use crate::twitch_stream_info::{GAsyncResult, GCancellable, GError};

const G_IO_ERROR_FAILED: c_int = 0;
const G_IO_ERROR_TIMED_OUT: c_int = 24;
const JSON_NODE_OBJECT: c_int = 0;
const JSON_NODE_VALUE: c_int = 2;
const JSON_NODE_NULL: c_int = 3;
const G_USEC_PER_SEC: i64 = 1_000_000;

const TWITCH_DEVICE_URI: &[u8] = b"https://id.twitch.tv/oauth2/device\0";
const TWITCH_TOKEN_URI: &[u8] = b"https://id.twitch.tv/oauth2/token\0";
const TWITCH_FOLLOWS_SCOPE: &[u8] = b"user:read:follows\0";
const TWITCH_DEVICE_GRANT: &[u8] = b"urn:ietf:params:oauth:grant-type:device_code\0";
const TWITCH_REFRESH_GRANT: &[u8] = b"refresh_token\0";

pub struct TwitchAuthDeviceCode {
    pub device_code: *mut c_char,
    pub user_code: *mut c_char,
    pub verification_uri: *mut c_char,
    pub expires_in: c_uint,
    pub interval: c_uint,
}

pub struct TwitchAuthToken {
    pub access_token: *mut c_char,
    pub refresh_token: *mut c_char,
    pub expires_in: c_uint,
}

struct DeviceCodeData {
    client_id: *mut c_char,
}

struct PollTokenData {
    client_id: *mut c_char,
    device_code: *mut c_char,
    expires_in: c_uint,
    interval: c_uint,
}

struct AuthResponse {
    status: c_uint,
    body: *mut c_char,
}

#[repr(C)]
pub struct GTask {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GBytes {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupSession {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupMessage {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupMessageHeaders {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonParser {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonNode {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonObject {
    _private: [u8; 0],
}

pub type GAsyncReadyCallback =
    Option<unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void)>;
type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GTaskThreadFunc =
    unsafe extern "C" fn(*mut GTask, *mut c_void, *mut c_void, *mut GCancellable);

unsafe extern "C" {
    fn g_bytes_get_data(bytes: *mut GBytes, size: *mut usize) -> *const c_void;
    fn g_bytes_new_static(data: *const c_void, size: usize) -> *mut GBytes;
    fn g_bytes_unref(bytes: *mut GBytes);
    fn g_cancellable_set_error_if_cancelled(
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> c_int;
    fn g_clear_error(error: *mut *mut GError);
    fn g_free(mem: *mut c_void);
    fn g_get_monotonic_time() -> i64;
    fn g_io_error_quark() -> c_uint;
    fn g_object_set(object: *mut c_void, first_property_name: *const c_char, ...);
    fn g_object_unref(object: *mut c_void);
    fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_strcmp0(str1: *const c_char, str2: *const c_char) -> c_int;
    fn g_strndup(str: *const c_char, n: usize) -> *mut c_char;
    fn g_uri_escape_string(
        unescaped: *const c_char,
        reserved_chars_allowed: *const c_char,
        allow_utf8: c_int,
    ) -> *mut c_char;
    fn g_usleep(microseconds: c_ulong);

    fn g_task_is_valid(result: *mut c_void, source_object: *mut c_void) -> c_int;
    fn g_task_new(
        source_object: *mut c_void,
        cancellable: *mut GCancellable,
        callback: GAsyncReadyCallback,
        callback_data: *mut c_void,
    ) -> *mut GTask;
    fn g_task_propagate_pointer(task: *mut GTask, error: *mut *mut GError) -> *mut c_void;
    fn g_task_return_error(task: *mut GTask, error: *mut GError);
    fn g_task_return_pointer(
        task: *mut GTask,
        result: *mut c_void,
        result_destroy: Option<GDestroyNotify>,
    );
    fn g_task_run_in_thread(task: *mut GTask, task_func: Option<GTaskThreadFunc>);
    fn g_task_set_task_data(
        task: *mut GTask,
        task_data: *mut c_void,
        task_data_destroy: Option<GDestroyNotify>,
    );

    fn json_node_get_int(node: *mut JsonNode) -> i64;
    fn json_node_get_node_type(node: *mut JsonNode) -> c_int;
    fn json_node_get_object(node: *mut JsonNode) -> *mut JsonObject;
    fn json_node_get_string(node: *mut JsonNode) -> *const c_char;
    fn json_object_get_member(object: *mut JsonObject, member_name: *const c_char)
        -> *mut JsonNode;
    fn json_parser_get_root(parser: *mut JsonParser) -> *mut JsonNode;
    fn json_parser_load_from_data(
        parser: *mut JsonParser,
        data: *const c_char,
        length: isize,
        error: *mut *mut GError,
    ) -> c_int;
    fn json_parser_new() -> *mut JsonParser;

    fn soup_message_get_request_headers(msg: *mut SoupMessage) -> *mut SoupMessageHeaders;
    fn soup_message_get_status(msg: *mut SoupMessage) -> c_uint;
    fn soup_message_headers_append(
        headers: *mut SoupMessageHeaders,
        name: *const c_char,
        value: *const c_char,
    );
    fn soup_message_new(method: *const c_char, uri_string: *const c_char) -> *mut SoupMessage;
    fn soup_message_set_request_body_from_bytes(
        msg: *mut SoupMessage,
        content_type: *const c_char,
        bytes: *mut GBytes,
    );
    fn soup_session_new() -> *mut SoupSession;
    fn soup_session_send_and_read(
        session: *mut SoupSession,
        msg: *mut SoupMessage,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut GBytes;
}

unsafe extern "C" fn device_code_data_free(data: *mut c_void) {
    if data.is_null() {
        return;
    }

    let data = Box::from_raw(data as *mut DeviceCodeData);
    g_free(data.client_id as *mut c_void);
}

unsafe extern "C" fn poll_token_data_free(data: *mut c_void) {
    if data.is_null() {
        return;
    }

    let data = Box::from_raw(data as *mut PollTokenData);
    g_free(data.client_id as *mut c_void);
    g_free(data.device_code as *mut c_void);
}

unsafe fn auth_response_free(response: *mut AuthResponse) {
    if response.is_null() {
        return;
    }

    let response = Box::from_raw(response);
    g_free(response.body as *mut c_void);
}

pub unsafe fn twitch_auth_device_code_free(code: *mut TwitchAuthDeviceCode) {
    if code.is_null() {
        return;
    }

    let code = Box::from_raw(code);
    g_free(code.device_code as *mut c_void);
    g_free(code.user_code as *mut c_void);
    g_free(code.verification_uri as *mut c_void);
}

pub unsafe fn twitch_auth_token_free(token: *mut TwitchAuthToken) {
    if token.is_null() {
        return;
    }

    let token = Box::from_raw(token);
    g_free(token.access_token as *mut c_void);
    g_free(token.refresh_token as *mut c_void);
}

unsafe fn append_form_pair(form: &mut Vec<u8>, name: *const c_char, value: *const c_char) {
    if !form.is_empty() {
        form.push(b'&');
    }

    let escaped_name = g_uri_escape_string(name, ptr::null(), 1);
    let empty = b"\0".as_ptr() as *const c_char;
    let escaped_value =
        g_uri_escape_string(if value.is_null() { empty } else { value }, ptr::null(), 1);

    form.extend_from_slice(CStr::from_ptr(escaped_name).to_bytes());
    form.push(b'=');
    form.extend_from_slice(CStr::from_ptr(escaped_value).to_bytes());

    g_free(escaped_name as *mut c_void);
    g_free(escaped_value as *mut c_void);
}

unsafe fn post_auth_form(
    uri: *const c_char,
    body: &[u8],
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut AuthResponse {
    let session = soup_session_new();
    let message = soup_message_new(b"POST\0".as_ptr() as *const c_char, uri);
    let body_bytes = g_bytes_new_static(body.as_ptr() as *const c_void, body.len());

    g_object_set(
        session as *mut c_void,
        b"timeout\0".as_ptr() as *const c_char,
        15,
        ptr::null::<c_char>(),
    );
    let request_headers = soup_message_get_request_headers(message);
    soup_message_headers_append(
        request_headers,
        b"Accept\0".as_ptr() as *const c_char,
        b"application/json\0".as_ptr() as *const c_char,
    );
    soup_message_set_request_body_from_bytes(
        message,
        b"application/x-www-form-urlencoded\0".as_ptr() as *const c_char,
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

unsafe fn parse_json_object(
    parser: *mut JsonParser,
    json: *const c_char,
    error: *mut *mut GError,
) -> *mut JsonObject {
    if json_parser_load_from_data(parser, json, -1, error) == 0 {
        return ptr::null_mut();
    }

    let root = json_parser_get_root(parser);
    if root.is_null() || json_node_get_node_type(root) != JSON_NODE_OBJECT {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            b"Twitch returned invalid JSON\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    json_node_get_object(root)
}

unsafe fn json_string_or_null(object: *mut JsonObject, name: *const c_char) -> *const c_char {
    let node = json_object_get_member(object, name);
    if node.is_null() {
        return ptr::null();
    }

    let node_type = json_node_get_node_type(node);
    if node_type == JSON_NODE_NULL || node_type != JSON_NODE_VALUE {
        return ptr::null();
    }

    json_node_get_string(node)
}

unsafe fn json_uint_or_zero(object: *mut JsonObject, name: *const c_char) -> c_uint {
    let node = json_object_get_member(object, name);
    if node.is_null() {
        return 0;
    }

    let node_type = json_node_get_node_type(node);
    if node_type == JSON_NODE_NULL || node_type != JSON_NODE_VALUE {
        return 0;
    }

    let value = json_node_get_int(node);
    if value > 0 && value <= c_uint::MAX as i64 {
        value as c_uint
    } else {
        0
    }
}

unsafe fn parse_auth_error_message(json: *const c_char) -> *mut c_char {
    let parser = json_parser_new();
    let mut error: *mut GError = ptr::null_mut();
    let object = parse_json_object(parser, json, &mut error);
    if object.is_null() {
        g_clear_error(&mut error);
        g_object_unref(parser as *mut c_void);
        return g_strdup(b"\0".as_ptr() as *const c_char);
    }

    let message = json_string_or_null(object, b"message\0".as_ptr() as *const c_char);
    let result = if !message.is_null() && *message != 0 {
        g_strdup(message)
    } else {
        let error_name = json_string_or_null(object, b"error\0".as_ptr() as *const c_char);
        if error_name.is_null() {
            g_strdup(b"\0".as_ptr() as *const c_char)
        } else {
            g_strdup(error_name)
        }
    };
    g_object_unref(parser as *mut c_void);
    result
}

unsafe fn parse_device_code_response(
    json: *const c_char,
    error: *mut *mut GError,
) -> *mut TwitchAuthDeviceCode {
    let parser = json_parser_new();
    let object = parse_json_object(parser, json, error);
    if object.is_null() {
        g_object_unref(parser as *mut c_void);
        return ptr::null_mut();
    }

    let device_code = json_string_or_null(object, b"device_code\0".as_ptr() as *const c_char);
    let user_code = json_string_or_null(object, b"user_code\0".as_ptr() as *const c_char);
    let verification_uri =
        json_string_or_null(object, b"verification_uri\0".as_ptr() as *const c_char);
    if device_code.is_null() || user_code.is_null() || verification_uri.is_null() {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            b"Twitch did not return a device code\0".as_ptr() as *const c_char,
        );
        g_object_unref(parser as *mut c_void);
        return ptr::null_mut();
    }

    let mut code = Box::new(TwitchAuthDeviceCode {
        device_code: g_strdup(device_code),
        user_code: g_strdup(user_code),
        verification_uri: g_strdup(verification_uri),
        expires_in: json_uint_or_zero(object, b"expires_in\0".as_ptr() as *const c_char),
        interval: json_uint_or_zero(object, b"interval\0".as_ptr() as *const c_char),
    });
    if code.interval == 0 {
        code.interval = 5;
    }
    g_object_unref(parser as *mut c_void);
    Box::into_raw(code)
}

unsafe fn parse_token_response(
    json: *const c_char,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    let parser = json_parser_new();
    let object = parse_json_object(parser, json, error);
    if object.is_null() {
        g_object_unref(parser as *mut c_void);
        return ptr::null_mut();
    }

    let access_token = json_string_or_null(object, b"access_token\0".as_ptr() as *const c_char);
    if access_token.is_null() || *access_token == 0 {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            b"Twitch did not return an access token\0".as_ptr() as *const c_char,
        );
        g_object_unref(parser as *mut c_void);
        return ptr::null_mut();
    }

    let token = Box::new(TwitchAuthToken {
        access_token: g_strdup(access_token),
        refresh_token: g_strdup(json_string_or_null(
            object,
            b"refresh_token\0".as_ptr() as *const c_char,
        )),
        expires_in: json_uint_or_zero(object, b"expires_in\0".as_ptr() as *const c_char),
    });
    g_object_unref(parser as *mut c_void);
    Box::into_raw(token)
}

unsafe fn sleep_poll_interval(
    interval: c_uint,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> c_int {
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

unsafe fn request_device_code(
    data: *mut DeviceCodeData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchAuthDeviceCode {
    let mut form = Vec::new();
    append_form_pair(
        &mut form,
        b"client_id\0".as_ptr() as *const c_char,
        (*data).client_id,
    );
    append_form_pair(
        &mut form,
        b"scopes\0".as_ptr() as *const c_char,
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
            b"Twitch auth returned HTTP %u%s%s\0".as_ptr() as *const c_char,
            (*response).status,
            if *message != 0 {
                b": \0".as_ptr() as *const c_char
            } else {
                b"\0".as_ptr() as *const c_char
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

unsafe fn poll_device_token(
    data: *mut PollTokenData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    let mut interval = if (*data).interval > 0 {
        (*data).interval
    } else {
        5
    };
    let deadline_us = g_get_monotonic_time() + (*data).expires_in.max(1) as i64 * G_USEC_PER_SEC;

    while g_get_monotonic_time() < deadline_us {
        if g_cancellable_set_error_if_cancelled(cancel, error) != 0 {
            return ptr::null_mut();
        }

        let mut form = Vec::new();
        append_form_pair(
            &mut form,
            b"client_id\0".as_ptr() as *const c_char,
            (*data).client_id,
        );
        append_form_pair(
            &mut form,
            b"scope\0".as_ptr() as *const c_char,
            TWITCH_FOLLOWS_SCOPE.as_ptr() as *const c_char,
        );
        append_form_pair(
            &mut form,
            b"device_code\0".as_ptr() as *const c_char,
            (*data).device_code,
        );
        append_form_pair(
            &mut form,
            b"grant_type\0".as_ptr() as *const c_char,
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
        if g_strcmp0(
            message,
            b"authorization_pending\0".as_ptr() as *const c_char,
        ) == 0
        {
            g_free(message as *mut c_void);
            if sleep_poll_interval(interval, cancel, error) == 0 {
                return ptr::null_mut();
            }
            continue;
        }
        if g_strcmp0(message, b"slow_down\0".as_ptr() as *const c_char) == 0 {
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
            b"Twitch authorization failed%s%s\0".as_ptr() as *const c_char,
            if *message != 0 {
                b": \0".as_ptr() as *const c_char
            } else {
                b"\0".as_ptr() as *const c_char
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
        b"Twitch authorization timed out\0".as_ptr() as *const c_char,
    );
    ptr::null_mut()
}

pub unsafe fn twitch_auth_refresh_token(
    client_id: *const c_char,
    refresh_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    if client_id.is_null() || *client_id == 0 || refresh_token.is_null() || *refresh_token == 0 {
        return ptr::null_mut();
    }

    let mut form = Vec::new();
    append_form_pair(
        &mut form,
        b"client_id\0".as_ptr() as *const c_char,
        client_id,
    );
    append_form_pair(
        &mut form,
        b"grant_type\0".as_ptr() as *const c_char,
        TWITCH_REFRESH_GRANT.as_ptr() as *const c_char,
    );
    append_form_pair(
        &mut form,
        b"refresh_token\0".as_ptr() as *const c_char,
        refresh_token,
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

    if (*response).status < 200 || (*response).status >= 300 {
        let message = parse_auth_error_message((*response).body);
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            b"Twitch token refresh failed with HTTP %u%s%s\0".as_ptr() as *const c_char,
            (*response).status,
            if *message != 0 {
                b": \0".as_ptr() as *const c_char
            } else {
                b"\0".as_ptr() as *const c_char
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
            b"Twitch did not return a refresh token\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    token
}

unsafe extern "C" fn request_device_code_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
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

unsafe extern "C" fn poll_device_token_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
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

unsafe extern "C" fn twitch_auth_device_code_free_as_destroy(data: *mut c_void) {
    twitch_auth_device_code_free(data as *mut TwitchAuthDeviceCode);
}

unsafe extern "C" fn twitch_auth_token_free_as_destroy(data: *mut c_void) {
    twitch_auth_token_free(data as *mut TwitchAuthToken);
}

pub unsafe fn twitch_auth_request_device_code_async(
    client_id: *const c_char,
    cancel: *mut GCancellable,
    callback: GAsyncReadyCallback,
    user_data: *mut c_void,
) {
    if client_id.is_null() || *client_id == 0 {
        return;
    }

    let data = Box::new(DeviceCodeData {
        client_id: g_strdup(client_id),
    });
    let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
    g_task_set_task_data(
        task,
        Box::into_raw(data) as *mut c_void,
        Some(device_code_data_free),
    );
    g_task_run_in_thread(task, Some(request_device_code_worker));
    g_object_unref(task as *mut c_void);
}

pub unsafe fn twitch_auth_request_device_code_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut TwitchAuthDeviceCode {
    if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
        return ptr::null_mut();
    }

    g_task_propagate_pointer(result as *mut GTask, error) as *mut TwitchAuthDeviceCode
}

pub unsafe fn twitch_auth_poll_device_token_async(
    client_id: *const c_char,
    code: *const TwitchAuthDeviceCode,
    cancel: *mut GCancellable,
    callback: GAsyncReadyCallback,
    user_data: *mut c_void,
) {
    if client_id.is_null()
        || *client_id == 0
        || code.is_null()
        || (*code).device_code.is_null()
        || *(*code).device_code == 0
    {
        return;
    }

    let data = Box::new(PollTokenData {
        client_id: g_strdup(client_id),
        device_code: g_strdup((*code).device_code),
        expires_in: (*code).expires_in,
        interval: (*code).interval,
    });
    let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
    g_task_set_task_data(
        task,
        Box::into_raw(data) as *mut c_void,
        Some(poll_token_data_free),
    );
    g_task_run_in_thread(task, Some(poll_device_token_worker));
    g_object_unref(task as *mut c_void);
}

pub unsafe fn twitch_auth_poll_device_token_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
        return ptr::null_mut();
    }

    g_task_propagate_pointer(result as *mut GTask, error) as *mut TwitchAuthToken
}
