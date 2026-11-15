use gtk4::glib::ffi::g_strndup;
use std::ffi::c_char;

/// Copies through the first NUL; result owned, release with `g_free`.
pub(crate) fn dup_bytes(bytes: &[u8]) -> *mut c_char {
    // SAFETY: The slice is readable for its length; GLib copies and appends NUL.
    unsafe { g_strndup(bytes.as_ptr().cast(), bytes.len()) }
}

/// # Safety
/// - Non-null `value`: readable for at least one byte.
pub(crate) unsafe fn is_nonempty(value: *const c_char) -> bool {
    unsafe { !value.is_null() && *value != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn copies_empty_utf8_and_embedded_nul_strings() {
        for (input, expected) in [
            (b"".as_slice(), b"".as_slice()),
            ("äbc".as_bytes(), "äbc".as_bytes()),
            (b"a\0ignored".as_slice(), b"a".as_slice()),
        ] {
            let copy = dup_bytes(input);
            unsafe {
                assert_eq!(CStr::from_ptr(copy).to_bytes(), expected);
                gtk4::glib::ffi::g_free(copy.cast());
            }
        }
    }
}
