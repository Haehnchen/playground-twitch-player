use super::ffi::{
    G_IO_ERROR_FAILED, JsonObject, JsonParser, g_clear_error, g_io_error_quark, g_object_unref,
    g_set_error, g_strdup, json_node_get_int, json_node_get_node_type, json_node_get_object,
    json_node_get_string, json_object_get_member, json_parser_get_root, json_parser_load_from_data,
    json_parser_new,
};
use super::{TwitchAuthDeviceCode, TwitchAuthToken};
use crate::twitch::stream_info::GError;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;
pub(super) const JSON_NODE_OBJECT: c_int = 0;
pub(super) const JSON_NODE_VALUE: c_int = 2;
pub(super) const JSON_NODE_NULL: c_int = 3;

pub(super) unsafe fn parse_json_object(
    parser: *mut JsonParser,
    json: *const c_char,
    error: *mut *mut GError,
) -> *mut JsonObject {
    unsafe {
        if json_parser_load_from_data(parser, json, -1, error) == 0 {
            return ptr::null_mut();
        }

        let root = json_parser_get_root(parser);
        if root.is_null() || json_node_get_node_type(root) != JSON_NODE_OBJECT {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch returned invalid JSON".as_ptr(),
            );
            return ptr::null_mut();
        }

        json_node_get_object(root)
    }
}

pub(super) unsafe fn json_string_or_null(
    object: *mut JsonObject,
    name: *const c_char,
) -> *const c_char {
    unsafe {
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
}

pub(super) unsafe fn json_uint_or_zero(object: *mut JsonObject, name: *const c_char) -> c_uint {
    unsafe {
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
}

pub(super) unsafe fn parse_auth_error_message(json: *const c_char) -> *mut c_char {
    unsafe {
        let parser = json_parser_new();
        let mut error: *mut GError = ptr::null_mut();
        let object = parse_json_object(parser, json, &mut error);
        if object.is_null() {
            g_clear_error(&mut error);
            g_object_unref(parser as *mut c_void);
            return g_strdup(c"".as_ptr());
        }

        let message = json_string_or_null(object, c"message".as_ptr());
        let result = if !message.is_null() && *message != 0 {
            g_strdup(message)
        } else {
            let error_name = json_string_or_null(object, c"error".as_ptr());
            if error_name.is_null() {
                g_strdup(c"".as_ptr())
            } else {
                g_strdup(error_name)
            }
        };
        g_object_unref(parser as *mut c_void);
        result
    }
}

pub(super) unsafe fn parse_device_code_response(
    json: *const c_char,
    error: *mut *mut GError,
) -> *mut TwitchAuthDeviceCode {
    unsafe {
        let parser = json_parser_new();
        let object = parse_json_object(parser, json, error);
        if object.is_null() {
            g_object_unref(parser as *mut c_void);
            return ptr::null_mut();
        }

        let device_code = json_string_or_null(object, c"device_code".as_ptr());
        let user_code = json_string_or_null(object, c"user_code".as_ptr());
        let verification_uri = json_string_or_null(object, c"verification_uri".as_ptr());
        if device_code.is_null() || user_code.is_null() || verification_uri.is_null() {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch did not return a device code".as_ptr(),
            );
            g_object_unref(parser as *mut c_void);
            return ptr::null_mut();
        }

        let mut code = Box::new(TwitchAuthDeviceCode {
            device_code: g_strdup(device_code),
            user_code: g_strdup(user_code),
            verification_uri: g_strdup(verification_uri),
            expires_in: json_uint_or_zero(object, c"expires_in".as_ptr()),
            interval: json_uint_or_zero(object, c"interval".as_ptr()),
        });
        if code.interval == 0 {
            code.interval = 5;
        }
        g_object_unref(parser as *mut c_void);
        Box::into_raw(code)
    }
}

pub(super) unsafe fn parse_token_response(
    json: *const c_char,
    error: *mut *mut GError,
) -> *mut TwitchAuthToken {
    unsafe {
        let parser = json_parser_new();
        let object = parse_json_object(parser, json, error);
        if object.is_null() {
            g_object_unref(parser as *mut c_void);
            return ptr::null_mut();
        }

        let access_token = json_string_or_null(object, c"access_token".as_ptr());
        if access_token.is_null() || *access_token == 0 {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"Twitch did not return an access token".as_ptr(),
            );
            g_object_unref(parser as *mut c_void);
            return ptr::null_mut();
        }

        let token = Box::new(TwitchAuthToken {
            access_token: g_strdup(access_token),
            refresh_token: g_strdup(json_string_or_null(object, c"refresh_token".as_ptr())),
            expires_in: json_uint_or_zero(object, c"expires_in".as_ptr()),
        });
        g_object_unref(parser as *mut c_void);
        Box::into_raw(token)
    }
}
