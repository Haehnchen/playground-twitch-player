use super::ffi::{
    G_LOG_LEVEL_WARNING, G_PRIORITY_DEFAULT, GObject, GTypeInstance, g_free, g_idle_add_full,
    g_log, g_main_context_find_source_by_id, g_object_add_weak_pointer,
    g_object_remove_weak_pointer, g_object_unref, g_signal_connect_data, g_source_destroy,
    g_timeout_add_full, g_timeout_add_seconds_full, g_type_check_instance_is_a,
};
use super::playback::on_mpv_wakeup;
use super::{
    FALSE, G_LOG_DOMAIN, GSourceFunc, GType, PlayerSurface, StreamTile, TRUE, player_surface_ref,
    player_surface_unref,
};
use crate::player::session::player_session_set_wakeup_callback;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

pub(super) unsafe fn clear_pointer<T>(slot: *mut *mut T) {
    unsafe {
        if !(*slot).is_null() {
            let old = *slot;
            *slot = ptr::null_mut();
            g_free(old as *mut c_void);
        }
    }
}

pub(super) unsafe fn clear_object<T>(slot: *mut *mut T) {
    unsafe {
        if !(*slot).is_null() {
            let old = *slot;
            *slot = ptr::null_mut();
            g_object_unref(old as *mut c_void);
        }
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

pub(super) unsafe fn tile_ref(tile: *mut StreamTile) -> *mut StreamTile {
    unsafe {
        if !tile.is_null() {
            player_surface_ref((*tile).app);
        }
        tile
    }
}

pub(super) unsafe extern "C" fn tile_source_data_free(data: *mut c_void) {
    unsafe {
        let tile = data as *mut StreamTile;
        if !tile.is_null() {
            player_surface_unref((*tile).app);
        }
    }
}

pub(super) unsafe extern "C" fn surface_source_data_free(data: *mut c_void) {
    unsafe {
        player_surface_unref(data as *mut PlayerSurface);
    }
}

pub(super) unsafe extern "C" fn tile_signal_data_free(data: *mut c_void, _closure: *mut c_void) {
    unsafe {
        tile_source_data_free(data);
    }
}

pub(super) unsafe extern "C" fn tile_signal_data_ref(data: *mut c_void) -> *mut c_void {
    unsafe { tile_ref(data as *mut StreamTile) as *mut c_void }
}

pub(super) unsafe fn connect_tile_signal(
    instance: *mut c_void,
    detailed_signal: *const c_char,
    handler: *const c_void,
    tile: *mut StreamTile,
) {
    unsafe {
        g_signal_connect_data(
            instance,
            detailed_signal,
            handler,
            tile_ref(tile) as *mut c_void,
            Some(tile_signal_data_free),
            0,
        );
    }
}

pub(super) unsafe fn add_tile_idle(
    priority: c_int,
    function: Option<GSourceFunc>,
    tile: *mut StreamTile,
) -> c_uint {
    unsafe {
        g_idle_add_full(
            priority,
            function,
            tile_ref(tile) as *mut c_void,
            Some(tile_source_data_free),
        )
    }
}

pub(super) unsafe fn add_tile_timeout(
    interval: c_uint,
    function: Option<GSourceFunc>,
    tile: *mut StreamTile,
) -> c_uint {
    unsafe {
        g_timeout_add_full(
            G_PRIORITY_DEFAULT,
            interval,
            function,
            tile_ref(tile) as *mut c_void,
            Some(tile_source_data_free),
        )
    }
}

pub(super) unsafe fn add_surface_timeout(
    interval: c_uint,
    function: Option<GSourceFunc>,
    state: *mut PlayerSurface,
) -> c_uint {
    unsafe {
        g_timeout_add_full(
            G_PRIORITY_DEFAULT,
            interval,
            function,
            player_surface_ref(state) as *mut c_void,
            Some(surface_source_data_free),
        )
    }
}

pub(super) unsafe fn add_surface_seconds_timeout(
    interval: c_uint,
    function: Option<GSourceFunc>,
    state: *mut PlayerSurface,
) -> c_uint {
    unsafe {
        g_timeout_add_seconds_full(
            G_PRIORITY_DEFAULT,
            interval,
            function,
            player_surface_ref(state) as *mut c_void,
            Some(surface_source_data_free),
        )
    }
}

pub(super) unsafe fn tile_is_closing(tile: *mut StreamTile) -> bool {
    unsafe {
        tile.is_null() || (*tile).app.is_null() || (*(*tile).app).closing.load(Ordering::Acquire)
    }
}

pub(super) unsafe fn enable_tile_wakeup_callback(tile: *mut StreamTile) {
    unsafe {
        if (*tile).session.is_null() {
            return;
        }

        if (*tile).wakeup_callback_ref_held == 0 {
            player_surface_ref((*tile).app);
            (*tile).wakeup_callback_ref_held = TRUE;
        }
        player_session_set_wakeup_callback(
            (*tile).session,
            Some(on_mpv_wakeup),
            tile as *mut c_void,
        );
    }
}

pub(super) unsafe fn disable_tile_wakeup_callback(tile: *mut StreamTile) {
    unsafe {
        if !(*tile).session.is_null() {
            player_session_set_wakeup_callback((*tile).session, None, ptr::null_mut());
        }
        if (*tile).wakeup_callback_ref_held != 0 {
            (*tile).wakeup_callback_ref_held = FALSE;
            player_surface_unref((*tile).app);
        }
    }
}

pub(super) unsafe fn is_instance<T>(instance: *mut T, type_: GType) -> bool {
    unsafe {
        !instance.is_null()
            && g_type_check_instance_is_a(instance as *mut GTypeInstance, type_) != 0
    }
}

pub(super) unsafe fn log_warning(format: *const c_char, message: *const c_char) {
    unsafe {
        g_log(
            G_LOG_DOMAIN.as_ptr() as *const c_char,
            G_LOG_LEVEL_WARNING,
            format,
            message,
        );
    }
}

pub(super) unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    unsafe {
        if *source_id == 0 {
            return;
        }

        let source = g_main_context_find_source_by_id(ptr::null_mut(), *source_id);
        if !source.is_null() {
            g_source_destroy(source);
        }
        *source_id = 0;
    }
}
