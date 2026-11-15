//! GLib callbacks and reference helpers.

use super::ffi::*;
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_ref, channel_switcher_overlay_unref};
use crate::twitch_stream_info::{GError, GPtrArray};
use std::ffi::{c_char, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

pub(super) const G_LOG_DOMAIN: &[u8] = b"channel-switcher-overlay\0";

pub(super) unsafe extern "C" fn g_free_destroy(data: *mut c_void) {
    unsafe {
        g_free(data);
    }
}

pub(super) unsafe extern "C" fn g_object_unref_destroy(data: *mut c_void) {
    unsafe {
        g_object_unref(data);
    }
}

pub(super) unsafe extern "C" fn g_ptr_array_unref_destroy(data: *mut c_void) {
    unsafe {
        g_ptr_array_unref(data as *mut GPtrArray);
    }
}

pub(super) unsafe fn ptr_array_index<T>(array: *mut GPtrArray, index: c_uint) -> *mut T {
    unsafe { *(*array).pdata.add(index as usize) as *mut T }
}

pub(super) unsafe fn clear_object<T>(slot: *mut *mut T) {
    unsafe {
        if !(*slot).is_null() {
            g_object_unref((*slot) as *mut c_void);
            *slot = ptr::null_mut();
        }
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

unsafe extern "C" fn channel_switcher_source_data_free(data: *mut c_void) {
    unsafe {
        channel_switcher_overlay_unref(data as *mut ChannelSwitcherOverlay);
    }
}

unsafe extern "C" fn channel_switcher_signal_data_free(data: *mut c_void, _closure: *mut c_void) {
    unsafe {
        channel_switcher_overlay_unref(data as *mut ChannelSwitcherOverlay);
    }
}

pub(super) unsafe fn connect_switcher_signal(
    instance: *mut c_void,
    detailed_signal: *const c_char,
    handler: *const c_void,
    switcher: *mut ChannelSwitcherOverlay,
) {
    unsafe {
        g_signal_connect_data(
            instance,
            detailed_signal,
            handler,
            channel_switcher_overlay_ref(switcher) as *mut c_void,
            Some(channel_switcher_signal_data_free),
            0,
        );
    }
}

// Hold a state reference until GLib removes the timeout.
pub(super) unsafe fn add_switcher_timeout(
    interval: c_uint,
    function: Option<GSourceFunc>,
    switcher: *mut ChannelSwitcherOverlay,
) -> c_uint {
    unsafe {
        g_timeout_add_full(
            G_PRIORITY_DEFAULT,
            interval,
            function,
            channel_switcher_overlay_ref(switcher) as *mut c_void,
            Some(channel_switcher_source_data_free),
        )
    }
}

pub(super) unsafe fn bump_generation(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        (*switcher).generation = (*switcher).generation.wrapping_add(1);
    }
}

pub(super) unsafe fn switcher_is_closing(switcher: *mut ChannelSwitcherOverlay) -> bool {
    unsafe { switcher.is_null() || (*switcher).closing.load(Ordering::Acquire) }
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

pub(super) unsafe fn log_debug(format: *const c_char, first: *const c_char, second: *const c_char) {
    unsafe {
        g_log(
            G_LOG_DOMAIN.as_ptr() as *const c_char,
            G_LOG_LEVEL_DEBUG,
            format,
            first,
            second,
        );
    }
}

pub(super) unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    unsafe {
        if !source_id.is_null() && *source_id != 0 {
            g_source_remove(*source_id);
            *source_id = 0;
        }
    }
}

pub(super) unsafe fn add_weak_pointer<T>(object: *mut T, slot: *mut *mut T) {
    unsafe {
        g_object_add_weak_pointer(object as *mut GObject, slot as *mut *mut c_void);
    }
}
