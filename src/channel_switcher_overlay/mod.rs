//! Channel picker state and public API.

#![allow(clashing_extern_declarations)]

mod actions;
mod cards;
mod channels;
mod ffi;
mod images;
mod layout;
mod runtime;
mod style;
mod widgets;

#[cfg(test)]
mod tests;

use self::cards::{clear_grid, clear_preview_cards, show_status};
use self::channels::fetch_channel_list;
use self::ffi::*;
pub use self::ffi::{
    GBytes, GFile, GHashTable, GInputStream, GObject, GTask, GTypeInstance, GdkDisplay,
    GdkPaintable, GdkPixbuf, GdkTexture, GraphenePoint, GtkBox, GtkButton, GtkCssProvider,
    GtkEditable, GtkEntry, GtkGesture, GtkGestureClick, GtkGestureSingle, GtkGrid, GtkLabel,
    GtkOverlay, GtkPicture, GtkRoot, GtkScrolledWindow, GtkSearchEntry, GtkStyleProvider,
    GtkWidget,
};
use self::images::clear_image_cache;
use self::layout::position_panel;
use self::runtime::{
    bump_generation, clear_object, g_free_destroy, g_object_unref_destroy,
    g_ptr_array_unref_destroy, remove_source_if_active, remove_weak_pointer,
};
use self::style::install_css;
use self::widgets::build_widgets;
use crate::settings::{AppSettings, AppSettingsChannel};
use crate::twitch_stream_info::{GCancellable, GPtrArray};
use std::ffi::{c_char, c_double, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct ChannelSwitcherOverlay {
    overlay: *mut GtkOverlay,
    backdrop: *mut GtkWidget,
    panel: *mut GtkWidget,
    grid: *mut GtkWidget,
    scroller: *mut GtkWidget,
    search_entry: *mut GtkWidget,
    direct_channel_entry: *mut GtkWidget,

    settings: *mut AppSettings,

    previews: *mut GPtrArray,
    preview_cards: *mut GPtrArray,
    preview_card_columns: c_uint,
    preview_card_width: c_int,
    preview_width: c_int,
    preview_height: c_int,
    preview_card_build_source: c_uint,
    preview_card_next_index: c_uint,
    preview_card_visible_count: c_uint,

    image_cache: *mut GHashTable,
    image_waiters: *mut GHashTable,

    cached_channels_key: *mut c_char,
    cached_at_us: i64,
    cancel: *mut GCancellable,
    search_debounce_source: c_uint,
    generation: c_uint,

    activate_callback: ChannelSwitcherActivateCallback,
    user_data: *mut c_void,
    settings_callback: ChannelSwitcherSettingsCallback,
    settings_user_data: *mut c_void,

    ref_count: AtomicUsize,
    closing: AtomicBool,
}

pub type ChannelSwitcherActivateCallback =
    Option<unsafe extern "C" fn(*const AppSettingsChannel, *mut c_void)>;
pub type ChannelSwitcherSettingsCallback = Option<unsafe extern "C" fn(*mut c_void)>;

impl ChannelSwitcherOverlay {
    // Allocate state and caches before creating GTK widgets.
    fn new(
        settings: *mut AppSettings,
        activate_callback: ChannelSwitcherActivateCallback,
        user_data: *mut c_void,
        settings_callback: ChannelSwitcherSettingsCallback,
        settings_user_data: *mut c_void,
    ) -> Self {
        unsafe {
            Self {
                overlay: ptr::null_mut(),
                backdrop: ptr::null_mut(),
                panel: ptr::null_mut(),
                grid: ptr::null_mut(),
                scroller: ptr::null_mut(),
                search_entry: ptr::null_mut(),
                direct_channel_entry: ptr::null_mut(),

                settings,

                previews: ptr::null_mut(),
                preview_cards: g_ptr_array_new_with_free_func(Some(g_object_unref_destroy)),
                preview_card_columns: 0,
                preview_card_width: 0,
                preview_width: 0,
                preview_height: 0,
                preview_card_build_source: 0,
                preview_card_next_index: 0,
                preview_card_visible_count: 0,

                image_cache: g_hash_table_new_full(
                    Some(g_str_hash),
                    Some(g_str_equal),
                    Some(g_free_destroy),
                    Some(g_object_unref_destroy),
                ),
                image_waiters: g_hash_table_new_full(
                    Some(g_str_hash),
                    Some(g_str_equal),
                    Some(g_free_destroy),
                    Some(g_ptr_array_unref_destroy),
                ),

                cached_channels_key: ptr::null_mut(),
                cached_at_us: 0,
                cancel: ptr::null_mut(),
                search_debounce_source: 0,
                generation: 0,

                activate_callback,
                user_data,
                settings_callback,
                settings_user_data,

                ref_count: AtomicUsize::new(1),
                closing: AtomicBool::new(false),
            }
        }
    }
}

unsafe fn channel_switcher_overlay_ref(
    switcher: *mut ChannelSwitcherOverlay,
) -> *mut ChannelSwitcherOverlay {
    unsafe {
        if !switcher.is_null() {
            (*switcher).ref_count.fetch_add(1, Ordering::Relaxed);
        }
        switcher
    }
}

// Free state after the last signal or async callback releases it.
unsafe fn channel_switcher_overlay_unref(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if switcher.is_null() || (*switcher).ref_count.fetch_sub(1, Ordering::AcqRel) != 1 {
            return;
        }

        if !(*switcher).cancel.is_null() {
            g_cancellable_cancel((*switcher).cancel);
            clear_object(&mut (*switcher).cancel);
        }
        if !(*switcher).previews.is_null() {
            g_ptr_array_unref((*switcher).previews);
        }
        if !(*switcher).preview_cards.is_null() {
            g_ptr_array_unref((*switcher).preview_cards);
        }
        if !(*switcher).image_cache.is_null() {
            g_hash_table_destroy((*switcher).image_cache);
        }
        if !(*switcher).image_waiters.is_null() {
            g_hash_table_destroy((*switcher).image_waiters);
        }
        g_free((*switcher).cached_channels_key as *mut c_void);

        remove_weak_pointer(&mut (*switcher).direct_channel_entry);
        remove_weak_pointer(&mut (*switcher).search_entry);
        remove_weak_pointer(&mut (*switcher).grid);
        remove_weak_pointer(&mut (*switcher).scroller);
        remove_weak_pointer(&mut (*switcher).panel);
        remove_weak_pointer(&mut (*switcher).backdrop);
        remove_weak_pointer(&mut (*switcher).overlay);

        drop(Box::from_raw(switcher));
    }
}

/// Create the hidden picker and connect its input handlers.
///
/// # Safety
/// - GTK initialized; GTK main thread.
/// - `overlay`: live `GtkOverlay`, outliving switcher.
/// - Settings live until replacement/free; callback data outlives switcher.
/// - Callbacks: declared C ABI, no unwind.
pub unsafe fn channel_switcher_overlay_new<O>(
    overlay: *mut O,
    settings: *mut AppSettings,
    activate_callback: ChannelSwitcherActivateCallback,
    user_data: *mut c_void,
    settings_callback: ChannelSwitcherSettingsCallback,
    settings_user_data: *mut c_void,
) -> *mut ChannelSwitcherOverlay {
    unsafe {
        let overlay = overlay as *mut GtkOverlay;
        install_css();

        let switcher = Box::into_raw(Box::new(ChannelSwitcherOverlay::new(
            settings,
            activate_callback,
            user_data,
            settings_callback,
            settings_user_data,
        )));
        (*switcher).overlay = overlay;

        build_widgets(switcher);

        switcher
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `switcher`: null or live instance.
/// - `settings`: live until replacement/free; serialize mutations.
pub unsafe fn channel_switcher_overlay_set_settings(
    switcher: *mut ChannelSwitcherOverlay,
    settings: *mut AppSettings,
) {
    unsafe {
        if switcher.is_null() || (*switcher).closing.load(Ordering::Acquire) {
            return;
        }

        (*switcher).settings = settings;
        if !(*switcher).previews.is_null() {
            g_ptr_array_unref((*switcher).previews);
            (*switcher).previews = ptr::null_mut();
        }
        clear_preview_cards(switcher);
        g_free((*switcher).cached_channels_key as *mut c_void);
        (*switcher).cached_channels_key = ptr::null_mut();
        (*switcher).cached_at_us = 0;
    }
}

/// Open the picker and request the live channel list.
///
/// # Safety
/// - GTK initialized; GTK main thread.
/// - `switcher`: null or live instance.
pub unsafe fn channel_switcher_overlay_show_at(
    switcher: *mut ChannelSwitcherOverlay,
    _x: c_double,
    _y: c_double,
) {
    unsafe {
        if switcher.is_null()
            || (*switcher).closing.load(Ordering::Acquire)
            || (*switcher).settings.is_null()
            || (*switcher).panel.is_null()
        {
            return;
        }

        bump_generation(switcher);
        remove_source_if_active(&mut (*switcher).preview_card_build_source);
        remove_source_if_active(&mut (*switcher).search_debounce_source);
        if !(*switcher).search_entry.is_null() {
            gtk_editable_set_text((*switcher).search_entry as *mut GtkEditable, c"".as_ptr());
        }
        if !(*switcher).direct_channel_entry.is_null() {
            gtk_editable_set_text(
                (*switcher).direct_channel_entry as *mut GtkEditable,
                c"".as_ptr(),
            );
        }
        remove_source_if_active(&mut (*switcher).search_debounce_source);
        if !(*switcher).backdrop.is_null() {
            gtk_widget_set_visible((*switcher).backdrop, TRUE);
        }
        gtk_widget_set_visible((*switcher).panel, TRUE);
        position_panel(switcher);
        if !(*switcher).search_entry.is_null() {
            gtk_widget_grab_focus((*switcher).search_entry);
        }
        show_status(switcher, c"Loading live channels".as_ptr());

        fetch_channel_list(switcher);
    }
}

/// Cancel pending work and hide the picker.
///
/// # Safety
/// - GTK initialized; GTK main thread.
/// - `switcher`: null or live instance.
pub unsafe fn channel_switcher_overlay_hide(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if switcher.is_null() {
            return;
        }

        bump_generation(switcher);
        remove_source_if_active(&mut (*switcher).preview_card_build_source);
        remove_source_if_active(&mut (*switcher).search_debounce_source);
        if !(*switcher).cancel.is_null() {
            g_cancellable_cancel((*switcher).cancel);
            clear_object(&mut (*switcher).cancel);
        }
        if !(*switcher).panel.is_null() {
            gtk_widget_set_visible((*switcher).panel, FALSE);
        }
        if !(*switcher).backdrop.is_null() {
            gtk_widget_set_visible((*switcher).backdrop, FALSE);
        }
        if !(*switcher).direct_channel_entry.is_null() {
            gtk_editable_set_text(
                (*switcher).direct_channel_entry as *mut GtkEditable,
                c"".as_ptr(),
            );
        }
        clear_grid(switcher);
        clear_image_cache(switcher);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `switcher`: null or live instance.
pub unsafe fn channel_switcher_overlay_is_visible(switcher: *mut ChannelSwitcherOverlay) -> c_int {
    unsafe {
        (!switcher.is_null()
            && !(*switcher).closing.load(Ordering::Acquire)
            && !(*switcher).panel.is_null()
            && gtk_widget_get_visible((*switcher).panel) != 0) as c_int
    }
}

/// Detach widgets and release the owner's reference.
///
/// # Safety
/// - GTK initialized; GTK main thread.
/// - `switcher`: null or live instance.
/// - Release owner once; no subsequent pointer access.
pub unsafe fn channel_switcher_overlay_free(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if switcher.is_null() {
            return;
        }

        if (*switcher).closing.swap(true, Ordering::AcqRel) {
            return;
        }

        channel_switcher_overlay_hide(switcher);
        if !(*switcher).previews.is_null() {
            g_ptr_array_unref((*switcher).previews);
            (*switcher).previews = ptr::null_mut();
        }
        clear_preview_cards(switcher);
        g_free((*switcher).cached_channels_key as *mut c_void);
        (*switcher).cached_channels_key = ptr::null_mut();
        (*switcher).cached_at_us = 0;
        if !(*switcher).panel.is_null() && !(*switcher).overlay.is_null() {
            gtk_overlay_remove_overlay((*switcher).overlay, (*switcher).panel);
        }
        if !(*switcher).backdrop.is_null() && !(*switcher).overlay.is_null() {
            gtk_overlay_remove_overlay((*switcher).overlay, (*switcher).backdrop);
        }
        remove_weak_pointer(&mut (*switcher).direct_channel_entry);
        remove_weak_pointer(&mut (*switcher).search_entry);
        remove_weak_pointer(&mut (*switcher).grid);
        remove_weak_pointer(&mut (*switcher).scroller);
        remove_weak_pointer(&mut (*switcher).panel);
        remove_weak_pointer(&mut (*switcher).backdrop);
        remove_weak_pointer(&mut (*switcher).overlay);

        channel_switcher_overlay_unref(switcher);
    }
}
