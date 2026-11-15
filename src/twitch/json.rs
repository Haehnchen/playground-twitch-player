use crate::glib_strings::dup_bytes;
use gtk4::{gio, glib};
use serde_json::Value;
use std::ffi::{c_char, c_uint};
use std::ptr;

pub(super) fn invalid_response(message: &str) -> glib::Error {
    glib::Error::new(gio::IOErrorEnum::InvalidData, message)
}

pub(super) fn parse(json: &[u8]) -> Result<Value, glib::Error> {
    serde_json::from_slice(json)
        .map_err(|err| invalid_response(&format!("Twitch returned invalid JSON: {err}")))
}

// Native callers see strings only up to the first NUL.
pub(super) fn string(value: &Value) -> Option<&str> {
    value.as_str().map(|s| s.split('\0').next().unwrap_or(""))
}

pub(super) fn dup_string(value: &Value) -> *mut c_char {
    string(value).map_or(ptr::null_mut(), |s| dup_bytes(s.as_bytes()))
}

pub(super) fn uint(value: &Value) -> c_uint {
    value.as_u64().and_then(|n| n.try_into().ok()).unwrap_or(0)
}
