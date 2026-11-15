use super::ffi::{
    G_LOG_LEVEL_DEBUG, G_PRIORITY_DEFAULT, GObject, GTypeInstance, g_log,
    g_main_context_find_source_by_id, g_object_add_weak_pointer, g_object_remove_weak_pointer,
    g_signal_connect_data, g_source_destroy, g_timeout_add_full, g_type_check_instance_is_a,
};
use super::{AppState, GSourceFunc, GType, app_state_ref, app_state_unref};
use crate::settings::GError;
use std::ffi::{c_char, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;
pub(super) const G_LOG_DOMAIN: &[u8] = b"twitch-player\0";

pub(super) unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    unsafe {
        if source_id.is_null() || *source_id == 0 {
            return;
        }

        let source = g_main_context_find_source_by_id(ptr::null_mut(), *source_id);
        if !source.is_null() {
            g_source_destroy(source);
        }
        *source_id = 0;
    }
}

pub(super) unsafe extern "C" fn app_state_source_data_free(data: *mut c_void) {
    unsafe {
        app_state_unref(data as *mut AppState);
    }
}

pub(super) unsafe extern "C" fn app_state_signal_data_free(
    data: *mut c_void,
    _closure: *mut c_void,
) {
    unsafe {
        app_state_unref(data as *mut AppState);
    }
}

pub(super) unsafe fn connect_app_state_signal(
    instance: *mut c_void,
    detailed_signal: *const c_char,
    handler: *const c_void,
    state: *mut AppState,
) {
    unsafe {
        g_signal_connect_data(
            instance,
            detailed_signal,
            handler,
            app_state_ref(state) as *mut c_void,
            Some(app_state_signal_data_free),
            0,
        );
    }
}

pub(super) unsafe fn add_app_state_timeout(
    interval: c_uint,
    function: Option<GSourceFunc>,
    state: *mut AppState,
) -> c_uint {
    unsafe {
        g_timeout_add_full(
            G_PRIORITY_DEFAULT,
            interval,
            function,
            app_state_ref(state) as *mut c_void,
            Some(app_state_source_data_free),
        )
    }
}

pub(super) unsafe fn app_state_is_closing(state: *mut AppState) -> bool {
    unsafe { state.is_null() || (*state).closing.load(Ordering::Acquire) }
}

pub(super) unsafe fn is_instance<T>(instance: *mut T, type_: GType) -> bool {
    unsafe {
        !instance.is_null()
            && g_type_check_instance_is_a(instance as *mut GTypeInstance, type_) != 0
    }
}

pub(super) unsafe fn error_message(error: *mut GError) -> *const c_char {
    unsafe {
        if error.is_null() || (*error).message.is_null() {
            c"unknown error".as_ptr()
        } else {
            (*error).message
        }
    }
}

pub(super) unsafe fn log_debug_static(message: *const c_char) {
    unsafe {
        g_log(
            G_LOG_DOMAIN.as_ptr() as *const c_char,
            G_LOG_LEVEL_DEBUG,
            c"%s".as_ptr(),
            message,
        );
    }
}

pub(super) unsafe fn log_debug_error(format: *const c_char, message: *const c_char) {
    unsafe {
        g_log(
            G_LOG_DOMAIN.as_ptr() as *const c_char,
            G_LOG_LEVEL_DEBUG,
            format,
            message,
        );
    }
}

pub(super) unsafe fn add_weak_pointer<T>(object: *mut T, slot: *mut *mut T) {
    unsafe {
        g_object_add_weak_pointer(object as *mut GObject, slot as *mut *mut c_void);
    }
}

pub(super) unsafe fn remove_weak_pointer<T>(slot: *mut *mut T) {
    unsafe {
        if (*slot).is_null() {
            return;
        }

        g_object_remove_weak_pointer((*slot) as *mut GObject, slot as *mut *mut c_void);
        *slot = ptr::null_mut();
    }
}
