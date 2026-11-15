//! Channel picker state and public API.

mod actions;
mod avatar;
mod cache;
mod channels;
mod ffi;
mod images;
mod layout;
mod radial;
mod runtime;
mod style;
mod tooltip;
mod widgets;

#[cfg(test)]
mod tests;

use self::channels::fetch_channel_list;
use self::ffi::*;
pub use self::ffi::{
    GBytes, GFile, GInputStream, GObject, GTask, GTypeInstance, GdkDisplay, GdkPaintable,
    GdkPixbuf, GdkTexture, GraphenePoint, GtkBox, GtkButton, GtkCssProvider, GtkEditable, GtkEntry,
    GtkGesture, GtkGestureClick, GtkGestureSingle, GtkGrid, GtkLabel, GtkOverlay, GtkPicture,
    GtkRoot, GtkScrolledWindow, GtkSearchEntry, GtkStyleProvider, GtkWidget,
};
use self::images::clear_image_cache;
use self::layout::position_panel;
use self::runtime::{bump_generation, remove_source_if_active, remove_weak_pointer};
use self::style::install_css;
use self::widgets::build_widgets;
use self::widgets::clear_preview_cards;
use crate::settings::{AppSettings, AppSettingsChannel};
use crate::twitch::stream_info::TwitchStreamPreview;
use std::collections::HashMap;
use std::ffi::{c_double, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub(crate) use cache::SharedPickerCache;
pub(crate) use radial::{
    PREFETCH_RADIUS, channel_switcher_overlay_leave_center, channel_switcher_overlay_near_center,
};

pub struct ChannelSwitcherOverlay {
    overlay: *mut GtkOverlay,
    backdrop: *mut GtkWidget,
    panel: *mut GtkWidget,
    grid: *mut GtkWidget,
    scroller: *mut GtkWidget,
    search_entry: *mut GtkWidget,
    direct_channel_entry: *mut GtkWidget,
    radial: Option<radial::RadialPicker>,
    cache: SharedPickerCache,
    cache_revision: Option<u64>,

    settings: *mut AppSettings,

    previews: Option<Vec<TwitchStreamPreview>>,
    preview_cards: Vec<gtk4::Widget>,
    preview_card_columns: c_uint,
    preview_card_width: c_int,
    preview_width: c_int,
    preview_height: c_int,
    preview_card_build_source: c_uint,
    preview_card_next_index: c_uint,
    preview_card_visible_count: c_uint,

    image_cache: HashMap<images::ImageKey, gtk4::gdk::Texture>,
    image_waiters: HashMap<images::ImageKey, images::ImageWaiters>,
    image_cancel: Option<gtk4::gio::Cancellable>,

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
        Self {
            overlay: ptr::null_mut(),
            backdrop: ptr::null_mut(),
            panel: ptr::null_mut(),
            grid: ptr::null_mut(),
            scroller: ptr::null_mut(),
            search_entry: ptr::null_mut(),
            direct_channel_entry: ptr::null_mut(),
            radial: None,
            cache: SharedPickerCache::default(),
            cache_revision: None,

            settings,

            previews: None,
            preview_cards: Vec::new(),
            preview_card_columns: 0,
            preview_card_width: 0,
            preview_width: 0,
            preview_height: 0,
            preview_card_build_source: 0,
            preview_card_next_index: 0,
            preview_card_visible_count: 0,

            image_cache: HashMap::new(),
            image_waiters: HashMap::new(),
            image_cancel: None,

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

        (*switcher).previews = None;

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

/// Attach the window's cache before the picker's first request (GTK main thread).
pub(crate) unsafe fn channel_switcher_overlay_use_cache(
    switcher: *mut ChannelSwitcherOverlay,
    cache: SharedPickerCache,
) {
    unsafe {
        (*switcher).cache = cache;
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
        bump_generation(switcher);
        (*switcher).cache.invalidate();
        (*switcher).cache_revision = None;
        radial::reset(switcher);
        (*switcher).previews = None;
        clear_preview_cards(switcher);
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
        radial::hide(switcher);
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
        clear_preview_cards(switcher);
        radial::hide(switcher);
        images::cancel_image_loads(switcher);
        // Rebuild pictures on the next approach so interrupted downloads retry.
        // Completed avatars survive both picker modes; large previews can go.
        radial::reset(switcher);
        (*switcher)
            .image_cache
            .retain(|(_, width, height), _| width == height && *width <= 64);
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
        radial::detach(switcher);
        clear_image_cache(switcher);
        (*switcher).previews = None;
        clear_preview_cards(switcher);
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
