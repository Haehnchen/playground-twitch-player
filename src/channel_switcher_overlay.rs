#![allow(clashing_extern_declarations)]

use std::ffi::{c_char, c_double, c_int, c_uint, c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::player_icons::{player_settings_icon_new, player_window_icon_new};
use crate::settings::{
    app_settings_get_channel, app_settings_get_channel_count, AppSettings, AppSettingsChannel,
};
use crate::twitch_channel_list::{
    twitch_channel_list_fetch_async, twitch_channel_list_fetch_finish,
};
use crate::twitch_stream_info::{
    twitch_stream_info_fetch_live_channels_async, twitch_stream_info_fetch_live_channels_finish,
    twitch_stream_info_format_live_duration, twitch_stream_info_format_viewer_count,
    GAsyncReadyCallback, GAsyncResult, GCancellable, GError, GPtrArray, TwitchStreamPreview,
};

macro_rules! cstr {
    ($value:literal) => {
        concat!($value, "\0").as_ptr() as *const c_char
    };
}

const PANEL_MARGIN: c_int = 12;
const PANEL_TOP_SAFE_MARGIN: c_int = 48;
const PANEL_BOTTOM_MARGIN: c_int = 64;
const PANEL_EXTRA_VERTICAL_SPACE: c_int = 36;
const LIVE_CHANNELS_CACHE_SECONDS: i64 = 10;
const SEARCH_DEBOUNCE_MS: c_uint = 300;
const PANEL_MIN_WIDTH: c_int = 430;
const PANEL_MAX_WIDTH: c_int = 1300;
const PANEL_MAX_COLUMNS: c_uint = 4;
const PANEL_HORIZONTAL_PADDING: c_int = 16;
const PANEL_SCROLLBAR_RESERVE_WIDTH: c_int = 18;
const CARD_WIDTH: c_int = 226;
const CARD_HORIZONTAL_PADDING: c_int = 10;
const CARD_SPACING: c_int = 6;
const PREVIEW_WIDTH: c_int = 226;
const PREVIEW_HEIGHT: c_int = 127;
const AVATAR_SIZE: c_int = 24;
const CARD_OUTER_WIDTH: c_int = CARD_WIDTH + CARD_HORIZONTAL_PADDING;

const FALSE: c_int = 0;
const TRUE: c_int = 1;
const G_SOURCE_REMOVE: c_int = 0;
const G_USEC_PER_SEC: i64 = 1_000_000;
const G_IO_ERROR_FAILED: c_int = 0;
const G_IO_ERROR_INVALID_ARGUMENT: c_int = 13;
const G_IO_ERROR_CANCELLED: c_int = 19;
const G_LOG_LEVEL_DEBUG: c_int = 1 << 7;
const G_LOG_DOMAIN: &[u8] = b"channel-switcher-overlay\0";

const GTK_ALIGN_FILL: c_int = 0;
const GTK_ALIGN_START: c_int = 1;
const GTK_ALIGN_CENTER: c_int = 3;
const GTK_CONTENT_FIT_COVER: c_int = 2;
const GTK_ENTRY_ICON_SECONDARY: c_int = 1;
const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_OVERFLOW_HIDDEN: c_int = 1;
const GTK_POLICY_AUTOMATIC: c_int = 1;
const GTK_POLICY_NEVER: c_int = 2;
const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;
const GDK_BUTTON_PRIMARY: c_uint = 1;
const GDK_COLORSPACE_RGB: c_int = 0;
const GDK_INTERP_BILINEAR: c_int = 2;
const GDK_MEMORY_R8G8B8A8: c_int = 5;
const PANGO_ELLIPSIZE_END: c_int = 3;
const PLAYER_WINDOW_ICON_CLOSE: c_int = 2;

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
}

struct RemoteImageData {
    switcher: *mut ChannelSwitcherOverlay,
    generation: c_uint,
    url: *mut c_char,
    cache_key: *mut c_char,
    width: c_int,
    height: c_int,
}

struct LiveFetchCallbackData {
    switcher: *mut ChannelSwitcherOverlay,
    generation: c_uint,
}

struct ChannelListFetchCallbackData {
    switcher: *mut ChannelSwitcherOverlay,
    generation: c_uint,
}

#[repr(C)]
pub struct GraphenePoint {
    x: f32,
    y: f32,
}

#[repr(C)]
pub struct GBytes {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkDisplay {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkPaintable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkPixbuf {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkTexture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GFile {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GHashTable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GInputStream {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GTypeInstance {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkButton {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkCssProvider {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEditable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEntry {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkGesture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkGestureClick {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkGestureSingle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkGrid {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkOverlay {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkPicture {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkRoot {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkScrolledWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkSearchEntry {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkStyleProvider {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;
type GType = usize;
pub type ChannelSwitcherActivateCallback =
    Option<unsafe extern "C" fn(*const AppSettingsChannel, *mut c_void)>;
pub type ChannelSwitcherSettingsCallback = Option<unsafe extern "C" fn(*mut c_void)>;

static CSS_INSTALLED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    fn g_ascii_strcasecmp(str1: *const c_char, str2: *const c_char) -> c_int;
    fn g_ascii_strdown(str: *const c_char, len: isize) -> *mut c_char;
    fn g_bytes_new(data: *const c_void, size: usize) -> *mut GBytes;
    fn g_bytes_new_take(data: *mut c_void, size: usize) -> *mut GBytes;
    fn g_bytes_unref(bytes: *mut GBytes);
    fn g_cancellable_cancel(cancellable: *mut GCancellable);
    fn g_cancellable_new() -> *mut GCancellable;
    fn g_clear_error(error: *mut *mut GError);
    fn g_error_matches(error: *const GError, domain: c_uint, code: c_int) -> c_int;
    fn g_file_load_contents_async(
        file: *mut GFile,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        user_data: *mut c_void,
    );
    fn g_file_load_contents_finish(
        file: *mut GFile,
        result: *mut GAsyncResult,
        contents: *mut *mut c_char,
        length: *mut usize,
        etag_out: *mut *mut c_char,
        error: *mut *mut GError,
    ) -> c_int;
    fn g_file_new_for_uri(uri: *const c_char) -> *mut GFile;
    fn g_free(mem: *mut c_void);
    fn g_get_monotonic_time() -> i64;
    fn g_hash_table_insert(
        hash_table: *mut GHashTable,
        key: *mut c_void,
        value: *mut c_void,
    ) -> c_int;
    fn g_hash_table_lookup(hash_table: *mut GHashTable, key: *const c_void) -> *mut c_void;
    fn g_hash_table_new_full(
        hash_func: Option<unsafe extern "C" fn(*const c_void) -> c_uint>,
        key_equal_func: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
        key_destroy_func: Option<GDestroyNotify>,
        value_destroy_func: Option<GDestroyNotify>,
    ) -> *mut GHashTable;
    fn g_hash_table_remove(hash_table: *mut GHashTable, key: *const c_void) -> c_int;
    fn g_hash_table_remove_all(hash_table: *mut GHashTable);
    fn g_io_error_quark() -> c_uint;
    fn g_log(log_domain: *const c_char, log_level: c_int, format: *const c_char, ...);
    fn g_malloc0(n_bytes: usize) -> *mut c_void;
    fn g_memory_input_stream_new_from_bytes(bytes: *mut GBytes) -> *mut GInputStream;
    fn g_object_add_weak_pointer(object: *mut GObject, weak_pointer_location: *mut *mut c_void);
    fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    fn g_object_ref(object: *mut c_void) -> *mut c_void;
    fn g_object_ref_sink(object: *mut c_void) -> *mut c_void;
    fn g_object_set_data_full(
        object: *mut GObject,
        key: *const c_char,
        data: *mut c_void,
        destroy: Option<GDestroyNotify>,
    );
    fn g_object_unref(object: *mut c_void);
    fn g_ptr_array_add(array: *mut GPtrArray, data: *mut c_void);
    fn g_ptr_array_new_with_free_func(element_free_func: Option<GDestroyNotify>) -> *mut GPtrArray;
    fn g_ptr_array_ref(array: *mut GPtrArray) -> *mut GPtrArray;
    fn g_ptr_array_set_size(array: *mut GPtrArray, length: c_uint);
    fn g_ptr_array_unref(array: *mut GPtrArray);
    fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    fn g_signal_emit_by_name(instance: *mut c_void, detailed_signal: *const c_char, ...);
    fn g_source_remove(tag: c_uint) -> c_int;
    fn g_str_equal(v1: *const c_void, v2: *const c_void) -> c_int;
    fn g_str_hash(v: *const c_void) -> c_uint;
    fn g_strcmp0(str1: *const c_char, str2: *const c_char) -> c_int;
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_strdup_printf(format: *const c_char, ...) -> *mut c_char;
    fn g_strfreev(str_array: *mut *mut c_char);
    fn g_timeout_add(interval: c_uint, function: Option<GSourceFunc>, data: *mut c_void) -> c_uint;
    fn g_type_check_instance_is_a(instance: *mut GTypeInstance, iface_type: GType) -> c_int;
    fn g_utf8_casefold(str: *const c_char, len: isize) -> *mut c_char;

    fn gdk_display_get_default() -> *mut GdkDisplay;
    fn gdk_memory_texture_new(
        width: c_int,
        height: c_int,
        format: c_int,
        bytes: *mut GBytes,
        stride: usize,
    ) -> *mut GdkTexture;
    fn gdk_pixbuf_composite(
        src: *mut GdkPixbuf,
        dest: *mut GdkPixbuf,
        dest_x: c_int,
        dest_y: c_int,
        dest_width: c_int,
        dest_height: c_int,
        offset_x: c_double,
        offset_y: c_double,
        scale_x: c_double,
        scale_y: c_double,
        interp_type: c_int,
        overall_alpha: c_int,
    );
    fn gdk_pixbuf_fill(pixbuf: *mut GdkPixbuf, pixel: c_uint);
    fn gdk_pixbuf_get_height(pixbuf: *const GdkPixbuf) -> c_int;
    fn gdk_pixbuf_get_pixels(pixbuf: *const GdkPixbuf) -> *mut u8;
    fn gdk_pixbuf_get_rowstride(pixbuf: *const GdkPixbuf) -> c_int;
    fn gdk_pixbuf_get_width(pixbuf: *const GdkPixbuf) -> c_int;
    fn gdk_pixbuf_new(
        colorspace: c_int,
        has_alpha: c_int,
        bits_per_sample: c_int,
        width: c_int,
        height: c_int,
    ) -> *mut GdkPixbuf;
    fn gdk_pixbuf_new_from_stream(
        stream: *mut GInputStream,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut GdkPixbuf;

    fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    fn gtk_button_get_type() -> GType;
    fn gtk_button_new() -> *mut GtkWidget;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    fn gtk_css_provider_load_from_string(css_provider: *mut GtkCssProvider, string: *const c_char);
    fn gtk_css_provider_new() -> *mut GtkCssProvider;
    fn gtk_editable_get_text(editable: *mut GtkEditable) -> *const c_char;
    fn gtk_editable_set_text(editable: *mut GtkEditable, text: *const c_char);
    fn gtk_entry_new() -> *mut GtkWidget;
    fn gtk_entry_set_icon_from_icon_name(
        entry: *mut GtkEntry,
        icon_pos: c_int,
        icon_name: *const c_char,
    );
    fn gtk_entry_set_icon_tooltip_text(
        entry: *mut GtkEntry,
        icon_pos: c_int,
        tooltip: *const c_char,
    );
    fn gtk_entry_set_placeholder_text(entry: *mut GtkEntry, text: *const c_char);
    fn gtk_gesture_click_new() -> *mut GtkGesture;
    fn gtk_gesture_single_set_button(gesture: *mut GtkGestureSingle, button: c_uint);
    fn gtk_grid_attach(
        grid: *mut GtkGrid,
        child: *mut GtkWidget,
        column: c_int,
        row: c_int,
        width: c_int,
        height: c_int,
    );
    fn gtk_grid_new() -> *mut GtkWidget;
    fn gtk_grid_remove(grid: *mut GtkGrid, child: *mut GtkWidget);
    fn gtk_grid_set_column_spacing(grid: *mut GtkGrid, spacing: c_uint);
    fn gtk_grid_set_row_spacing(grid: *mut GtkGrid, spacing: c_uint);
    fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    fn gtk_label_set_ellipsize(label: *mut GtkLabel, mode: c_int);
    fn gtk_label_set_max_width_chars(label: *mut GtkLabel, n_chars: c_int);
    fn gtk_label_set_single_line_mode(label: *mut GtkLabel, single_line_mode: c_int);
    fn gtk_label_set_wrap(label: *mut GtkLabel, wrap: c_int);
    fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: f32);
    fn gtk_overlay_add_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    fn gtk_overlay_remove_overlay(overlay: *mut GtkOverlay, widget: *mut GtkWidget);
    fn gtk_picture_new() -> *mut GtkWidget;
    fn gtk_picture_set_can_shrink(picture: *mut GtkPicture, can_shrink: c_int);
    fn gtk_picture_set_content_fit(picture: *mut GtkPicture, content_fit: c_int);
    fn gtk_picture_set_paintable(picture: *mut GtkPicture, paintable: *mut GdkPaintable);
    fn gtk_scrolled_window_new() -> *mut GtkWidget;
    fn gtk_scrolled_window_set_child(
        scrolled_window: *mut GtkScrolledWindow,
        child: *mut GtkWidget,
    );
    fn gtk_scrolled_window_set_max_content_height(
        scrolled_window: *mut GtkScrolledWindow,
        height: c_int,
    );
    fn gtk_scrolled_window_set_max_content_width(
        scrolled_window: *mut GtkScrolledWindow,
        width: c_int,
    );
    fn gtk_scrolled_window_set_min_content_width(
        scrolled_window: *mut GtkScrolledWindow,
        width: c_int,
    );
    fn gtk_scrolled_window_set_policy(
        scrolled_window: *mut GtkScrolledWindow,
        hscrollbar_policy: c_int,
        vscrollbar_policy: c_int,
    );
    fn gtk_scrolled_window_set_propagate_natural_height(
        scrolled_window: *mut GtkScrolledWindow,
        propagate: c_int,
    );
    fn gtk_scrolled_window_set_propagate_natural_width(
        scrolled_window: *mut GtkScrolledWindow,
        propagate: c_int,
    );
    fn gtk_search_entry_new() -> *mut GtkWidget;
    fn gtk_search_entry_set_placeholder_text(entry: *mut GtkSearchEntry, text: *const c_char);
    fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut GtkStyleProvider,
        priority: c_uint,
    );
    fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut c_void);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_compute_point(
        widget: *mut GtkWidget,
        target: *mut GtkWidget,
        point: *const GraphenePoint,
        out_point: *mut GraphenePoint,
    ) -> c_int;
    fn gtk_widget_get_first_child(widget: *mut GtkWidget) -> *mut GtkWidget;
    fn gtk_widget_get_height(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_next_sibling(widget: *mut GtkWidget) -> *mut GtkWidget;
    fn gtk_widget_get_root(widget: *mut GtkWidget) -> *mut GtkRoot;
    fn gtk_widget_get_visible(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_get_width(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_grab_focus(widget: *mut GtkWidget) -> c_int;
    fn gtk_widget_set_focusable(widget: *mut GtkWidget, focusable: c_int);
    fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_margin_end(widget: *mut GtkWidget, margin: c_int);
    fn gtk_widget_set_margin_start(widget: *mut GtkWidget, margin: c_int);
    fn gtk_widget_set_margin_top(widget: *mut GtkWidget, margin: c_int);
    fn gtk_widget_set_overflow(widget: *mut GtkWidget, overflow: c_int);
    fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);

}

unsafe extern "C" fn g_free_destroy(data: *mut c_void) {
    g_free(data);
}

unsafe extern "C" fn g_object_unref_destroy(data: *mut c_void) {
    g_object_unref(data);
}

unsafe extern "C" fn g_ptr_array_unref_destroy(data: *mut c_void) {
    g_ptr_array_unref(data as *mut GPtrArray);
}

unsafe fn is_nonempty(value: *const c_char) -> bool {
    !value.is_null() && *value != 0
}

unsafe fn dup_bytes(bytes: &[u8]) -> *mut c_char {
    let mut value = Vec::with_capacity(bytes.len() + 1);
    value.extend_from_slice(bytes);
    value.push(0);
    g_strdup(value.as_ptr() as *const c_char)
}

unsafe fn ptr_array_index<T>(array: *mut GPtrArray, index: c_uint) -> *mut T {
    *(*array).pdata.add(index as usize) as *mut T
}

unsafe fn clear_object<T>(slot: *mut *mut T) {
    if !(*slot).is_null() {
        g_object_unref((*slot) as *mut c_void);
        *slot = ptr::null_mut();
    }
}

unsafe fn bump_generation(switcher: *mut ChannelSwitcherOverlay) {
    (*switcher).generation = (*switcher).generation.wrapping_add(1);
}

unsafe fn error_message(error: *mut GError) -> *const c_char {
    if error.is_null() || (*error).message.is_null() {
        cstr!("unknown error")
    } else {
        (*error).message
    }
}

unsafe fn log_debug(format: *const c_char, first: *const c_char, second: *const c_char) {
    g_log(
        G_LOG_DOMAIN.as_ptr() as *const c_char,
        G_LOG_LEVEL_DEBUG,
        format,
        first,
        second,
    );
}

unsafe fn install_css() {
    if CSS_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let provider = gtk_css_provider_new();
    let css = CString::new(
        ".channel-switcher-panel {\
          background: rgba(12, 12, 14, 0.88);\
          color: #ffffff;\
          padding: 8px 8px 10px 8px;\
          border-radius: 6px;\
          box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);\
        }\
        .channel-switcher-backdrop {\
          background: transparent;\
        }\
        .channel-switcher-header {\
          margin-bottom: 4px;\
        }\
        .channel-switcher-search {\
          background: rgba(255, 255, 255, 0.08);\
          color: #ffffff;\
          border-color: rgba(255, 255, 255, 0.10);\
          outline-color: transparent;\
          box-shadow: none;\
          font-size: 12px;\
          min-height: 22px;\
          min-width: 165px;\
          padding: 1px 7px;\
          border-radius: 4px;\
        }\
        .channel-switcher-header-separator {\
          color: rgba(255, 255, 255, 0.34);\
          font-size: 13px;\
          margin-left: 1px;\
          margin-right: 1px;\
        }\
        .channel-switcher-open-entry {\
          background: rgba(255, 255, 255, 0.08);\
          color: #ffffff;\
          border-color: rgba(255, 255, 255, 0.10);\
          outline-color: transparent;\
          box-shadow: none;\
          font-size: 12px;\
          min-height: 22px;\
          min-width: 165px;\
          padding: 1px 7px;\
          border-radius: 4px;\
        }\
        .channel-switcher-search selection {\
          background: rgba(145, 70, 255, 0.50);\
          color: #ffffff;\
        }\
        .channel-switcher-action,\
        .channel-switcher-close {\
          background: transparent;\
          background-image: none;\
          color: #ffffff;\
          border-color: transparent;\
          outline-color: transparent;\
          box-shadow: none;\
          min-width: 24px;\
          min-height: 22px;\
          padding: 1px 4px;\
          border-radius: 4px;\
        }\
        .channel-switcher-action:hover {\
          background: rgba(255, 255, 255, 0.16);\
        }\
        .channel-switcher-close:hover {\
          background: rgba(170, 36, 36, 0.90);\
          background-image: none;\
        }\
        .channel-switcher-scroller,\
        .channel-switcher-scroller viewport {\
          background: transparent;\
        }\
        .channel-switcher-grid {\
          background: transparent;\
        }\
        .channel-switcher-item {\
          background: rgba(255, 255, 255, 0.06);\
          color: #ffffff;\
          border-color: transparent;\
          outline-color: transparent;\
          box-shadow: none;\
          border-radius: 5px;\
          margin: 0;\
          padding: 5px;\
        }\
        .channel-switcher-item:hover {\
          background: rgba(255, 255, 255, 0.13);\
        }\
        .channel-switcher-preview {\
          background: rgba(255, 255, 255, 0.08);\
          border-radius: 4px;\
          min-width: 226px;\
          min-height: 127px;\
        }\
        .channel-switcher-avatar {\
          background: rgba(0, 0, 0, 0.55);\
          border-radius: 999px;\
          min-width: 24px;\
          min-height: 24px;\
          margin: 0;\
        }\
        .channel-switcher-name {\
          color: #ffffff;\
          font-weight: 700;\
          font-size: 13px;\
        }\
        .channel-switcher-title {\
          color: rgba(255, 255, 255, 0.86);\
          font-size: 12px;\
        }\
        .channel-switcher-meta {\
          color: rgba(255, 255, 255, 0.66);\
          font-size: 11px;\
        }\
        .channel-switcher-status {\
          color: rgba(255, 255, 255, 0.72);\
          padding: 12px;\
        }",
    )
    .expect("static CSS has no NUL bytes");
    gtk_css_provider_load_from_string(provider, css.as_ptr());
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        provider as *mut GtkStyleProvider,
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    g_object_unref(provider as *mut c_void);
}

unsafe fn remote_image_data_free(data: *mut RemoteImageData) {
    if data.is_null() {
        return;
    }

    let data = Box::from_raw(data);
    g_free(data.url as *mut c_void);
    g_free(data.cache_key as *mut c_void);
}

unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    if !source_id.is_null() && *source_id != 0 {
        g_source_remove(*source_id);
        *source_id = 0;
    }
}

unsafe fn clear_grid(switcher: *mut ChannelSwitcherOverlay) {
    if (*switcher).grid.is_null() {
        return;
    }

    let mut child = gtk_widget_get_first_child((*switcher).grid);
    while !child.is_null() {
        let next = gtk_widget_get_next_sibling(child);
        gtk_grid_remove((*switcher).grid as *mut GtkGrid, child);
        child = next;
    }
}

unsafe fn clear_preview_cards(switcher: *mut ChannelSwitcherOverlay) {
    clear_grid(switcher);
    if !(*switcher).preview_cards.is_null() {
        g_ptr_array_set_size((*switcher).preview_cards, 0);
    }
    (*switcher).preview_card_columns = 0;
    (*switcher).preview_card_width = 0;
    (*switcher).preview_width = 0;
    (*switcher).preview_height = 0;
}

unsafe fn clear_image_cache(switcher: *mut ChannelSwitcherOverlay) {
    if !(*switcher).image_cache.is_null() {
        g_hash_table_remove_all((*switcher).image_cache);
    }
    if !(*switcher).image_waiters.is_null() {
        g_hash_table_remove_all((*switcher).image_waiters);
    }
}

unsafe fn set_remote_image_texture(image: *mut GtkWidget, texture: *mut GdkTexture) {
    if image.is_null() || texture.is_null() {
        return;
    }

    gtk_picture_set_paintable(image as *mut GtkPicture, texture as *mut GdkPaintable);
}

unsafe fn build_image_cache_key(url: *const c_char, width: c_int, height: c_int) -> *mut c_char {
    g_strdup_printf(cstr!("%s\n%d:%d"), url, width, height)
}

unsafe fn create_cover_texture_from_bytes(
    bytes: *mut GBytes,
    width: c_int,
    height: c_int,
    error: *mut *mut GError,
) -> *mut GdkTexture {
    if width <= 0 || height <= 0 {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_INVALID_ARGUMENT,
            cstr!("invalid image target size"),
        );
        return ptr::null_mut();
    }

    let stream = g_memory_input_stream_new_from_bytes(bytes);
    let source = gdk_pixbuf_new_from_stream(stream, ptr::null_mut(), error);
    g_object_unref(stream as *mut c_void);
    if source.is_null() {
        return ptr::null_mut();
    }

    let source_width = gdk_pixbuf_get_width(source);
    let source_height = gdk_pixbuf_get_height(source);
    if source_width <= 0 || source_height <= 0 {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            cstr!("invalid image dimensions"),
        );
        g_object_unref(source as *mut c_void);
        return ptr::null_mut();
    }

    let scale = (width as c_double / source_width as c_double)
        .max(height as c_double / source_height as c_double);
    let scaled_width = source_width as c_double * scale;
    let scaled_height = source_height as c_double * scale;
    let offset_x = (width as c_double - scaled_width) / 2.0;
    let offset_y = (height as c_double - scaled_height) / 2.0;

    let target = gdk_pixbuf_new(GDK_COLORSPACE_RGB, TRUE, 8, width, height);
    if target.is_null() {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            cstr!("could not allocate image target"),
        );
        g_object_unref(source as *mut c_void);
        return ptr::null_mut();
    }

    gdk_pixbuf_fill(target, 0x00000000);
    gdk_pixbuf_composite(
        source,
        target,
        0,
        0,
        width,
        height,
        offset_x,
        offset_y,
        scale,
        scale,
        GDK_INTERP_BILINEAR,
        255,
    );

    let stride = gdk_pixbuf_get_rowstride(target) as usize;
    let length = stride * height as usize;
    let texture_bytes = g_bytes_new(gdk_pixbuf_get_pixels(target) as *const c_void, length);
    let texture = gdk_memory_texture_new(width, height, GDK_MEMORY_R8G8B8A8, texture_bytes, stride);
    g_bytes_unref(texture_bytes);
    g_object_unref(target as *mut c_void);
    g_object_unref(source as *mut c_void);
    texture
}

unsafe fn create_placeholder_texture(width: c_int, height: c_int) -> *mut GdkTexture {
    if width <= 0 || height <= 0 {
        return ptr::null_mut();
    }

    let stride = width as usize * 4;
    let length = stride * height as usize;
    let bytes = g_bytes_new_take(g_malloc0(length), length);
    let texture = gdk_memory_texture_new(width, height, GDK_MEMORY_R8G8B8A8, bytes, stride);
    g_bytes_unref(bytes);
    texture
}

unsafe fn live_fetch_callback_data_free(data: *mut LiveFetchCallbackData) {
    if !data.is_null() {
        drop(Box::from_raw(data));
    }
}

unsafe extern "C" fn on_remote_image_loaded(
    source: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let data = user_data as *mut RemoteImageData;
    let switcher = (*data).switcher;
    let mut error: *mut GError = ptr::null_mut();
    let mut contents: *mut c_char = ptr::null_mut();
    let mut length: usize = 0;

    if (*switcher).image_waiters.is_null() {
        remote_image_data_free(data);
        return;
    }

    let waiters = g_hash_table_lookup(
        (*switcher).image_waiters,
        (*data).cache_key as *const c_void,
    ) as *mut GPtrArray;
    if !waiters.is_null() {
        g_ptr_array_ref(waiters);
        g_hash_table_remove(
            (*switcher).image_waiters,
            (*data).cache_key as *const c_void,
        );
    }

    if g_file_load_contents_finish(
        source as *mut GFile,
        result,
        &mut contents,
        &mut length,
        ptr::null_mut(),
        &mut error,
    ) == 0
    {
        log_debug(
            cstr!("image load failed for %s: %s"),
            (*data).url,
            error_message(error),
        );
        if !waiters.is_null() {
            g_ptr_array_unref(waiters);
        }
        g_clear_error(&mut error);
        remote_image_data_free(data);
        return;
    }

    let bytes = g_bytes_new_take(contents as *mut c_void, length);
    if (*data).generation != (*switcher).generation || (*switcher).panel.is_null() {
        if !waiters.is_null() {
            g_ptr_array_unref(waiters);
        }
        g_bytes_unref(bytes);
        remote_image_data_free(data);
        return;
    }

    let texture = create_cover_texture_from_bytes(bytes, (*data).width, (*data).height, &mut error);
    g_bytes_unref(bytes);
    if !texture.is_null() {
        if !(*switcher).image_cache.is_null() {
            g_hash_table_insert(
                (*switcher).image_cache,
                g_strdup((*data).cache_key) as *mut c_void,
                g_object_ref(texture as *mut c_void),
            );
        }
        if !waiters.is_null() {
            for i in 0..(*waiters).len {
                set_remote_image_texture(ptr_array_index(waiters, i), texture);
            }
        }
        g_object_unref(texture as *mut c_void);
    } else if !error.is_null() {
        log_debug(
            cstr!("image decode failed for %s: %s"),
            (*data).url,
            (*error).message,
        );
        g_clear_error(&mut error);
    }

    if !waiters.is_null() {
        g_ptr_array_unref(waiters);
    }

    remote_image_data_free(data);
}

unsafe fn load_remote_image(
    switcher: *mut ChannelSwitcherOverlay,
    image: *mut GtkWidget,
    url: *const c_char,
    width: c_int,
    height: c_int,
) {
    if !is_nonempty(url) || (*switcher).image_cache.is_null() || (*switcher).image_waiters.is_null()
    {
        return;
    }

    let cache_key = build_image_cache_key(url, width, height);
    let cached =
        g_hash_table_lookup((*switcher).image_cache, cache_key as *const c_void) as *mut GdkTexture;
    if !cached.is_null() {
        set_remote_image_texture(image, cached);
        g_free(cache_key as *mut c_void);
        return;
    }

    let mut waiters = g_hash_table_lookup((*switcher).image_waiters, cache_key as *const c_void)
        as *mut GPtrArray;
    if !waiters.is_null() {
        g_ptr_array_add(waiters, g_object_ref(image as *mut c_void));
        g_free(cache_key as *mut c_void);
        return;
    }

    waiters = g_ptr_array_new_with_free_func(Some(g_object_unref_destroy));
    g_ptr_array_add(waiters, g_object_ref(image as *mut c_void));
    g_hash_table_insert(
        (*switcher).image_waiters,
        g_strdup(cache_key) as *mut c_void,
        waiters as *mut c_void,
    );

    let data = Box::into_raw(Box::new(RemoteImageData {
        switcher,
        generation: (*switcher).generation,
        url: g_strdup(url),
        cache_key: g_strdup(cache_key),
        width,
        height,
    }));

    let file = g_file_new_for_uri(url);
    g_file_load_contents_async(
        file,
        ptr::null_mut(),
        Some(on_remote_image_loaded),
        data as *mut c_void,
    );
    g_object_unref(file as *mut c_void);
    g_free(cache_key as *mut c_void);
}

fn calculate_panel_width(overlay_width: c_int) -> c_int {
    let available_width = 1.max(overlay_width - PANEL_MARGIN * 2);
    let max_panel_width = PANEL_MAX_WIDTH.min(available_width);
    let min_panel_width = PANEL_MIN_WIDTH.min(available_width);

    max_panel_width.max(min_panel_width).min(max_panel_width)
}

fn calculate_grid_width(panel_width: c_int) -> c_int {
    1.max(panel_width - PANEL_HORIZONTAL_PADDING - PANEL_SCROLLBAR_RESERVE_WIDTH)
}

fn calculate_grid_columns(grid_width: c_int) -> c_uint {
    PANEL_MAX_COLUMNS
        .min(1.max((grid_width + CARD_SPACING) / (CARD_OUTER_WIDTH + CARD_SPACING)) as c_uint)
}

fn calculate_card_width(grid_width: c_int, columns: c_uint) -> c_int {
    let spacing_width = (if columns > 0 { columns - 1 } else { 0 }) as c_int * CARD_SPACING;
    let card_outer_width = (grid_width - spacing_width) / 1.max(columns as c_int);

    CARD_WIDTH.max(card_outer_width - CARD_HORIZONTAL_PADDING)
}

fn calculate_preview_height(preview_width: c_int) -> c_int {
    1.max((preview_width * 9 + 8) / 16)
}

unsafe fn calculate_card_layout(
    switcher: *mut ChannelSwitcherOverlay,
    columns: *mut c_uint,
    card_width: *mut c_int,
    preview_width: *mut c_int,
    preview_height: *mut c_int,
) {
    let mut panel_width = 0;

    if !(*switcher).overlay.is_null() {
        panel_width =
            calculate_panel_width(gtk_widget_get_width((*switcher).overlay as *mut GtkWidget));
    } else if !(*switcher).panel.is_null() {
        panel_width = gtk_widget_get_width((*switcher).panel);
    }

    let grid_width = calculate_grid_width(panel_width);
    let calculated_columns = calculate_grid_columns(grid_width);
    let calculated_card_width = calculate_card_width(grid_width, calculated_columns);

    if !columns.is_null() {
        *columns = calculated_columns;
    }
    if !card_width.is_null() {
        *card_width = calculated_card_width;
    }
    if !preview_width.is_null() {
        *preview_width = calculated_card_width;
    }
    if !preview_height.is_null() {
        *preview_height = calculate_preview_height(calculated_card_width);
    }
}

unsafe fn get_grid_columns(switcher: *mut ChannelSwitcherOverlay) -> c_uint {
    let mut columns = 1;
    calculate_card_layout(
        switcher,
        &mut columns,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
    );
    columns
}

unsafe fn show_status(switcher: *mut ChannelSwitcherOverlay, message: *const c_char) {
    if (*switcher).grid.is_null() {
        return;
    }

    clear_grid(switcher);

    let label = gtk_label_new(message);
    gtk_widget_add_css_class(label, cstr!("channel-switcher-status"));
    gtk_label_set_wrap(label as *mut GtkLabel, TRUE);
    gtk_label_set_xalign(label as *mut GtkLabel, 0.0);
    gtk_widget_set_hexpand(label, TRUE);
    gtk_grid_attach(
        (*switcher).grid as *mut GtkGrid,
        label,
        0,
        0,
        get_grid_columns(switcher) as c_int,
        1,
    );
}

unsafe fn position_panel(switcher: *mut ChannelSwitcherOverlay) {
    if (*switcher).overlay.is_null()
        || (*switcher).panel.is_null()
        || (*switcher).scroller.is_null()
    {
        return;
    }

    let overlay_width = gtk_widget_get_width((*switcher).overlay as *mut GtkWidget);
    let overlay_height = gtk_widget_get_height((*switcher).overlay as *mut GtkWidget);
    let mut root_y = PANEL_TOP_SAFE_MARGIN as c_double;
    let root = gtk_widget_get_root((*switcher).overlay as *mut GtkWidget);
    if !root.is_null() {
        let origin = GraphenePoint { x: 0.0, y: 0.0 };
        let mut root_point = GraphenePoint { x: 0.0, y: 0.0 };
        if gtk_widget_compute_point(
            (*switcher).overlay as *mut GtkWidget,
            root as *mut GtkWidget,
            &origin,
            &mut root_point,
        ) != 0
        {
            root_y = root_point.y as c_double;
        }
    }

    let top_margin = if root_y < PANEL_TOP_SAFE_MARGIN as c_double {
        PANEL_TOP_SAFE_MARGIN - root_y as c_int
    } else {
        PANEL_MARGIN
    };
    let panel_width = calculate_panel_width(overlay_width);
    let grid_width = calculate_grid_width(panel_width);
    let scroller_height =
        1.max(overlay_height - top_margin - PANEL_BOTTOM_MARGIN - PANEL_EXTRA_VERTICAL_SPACE);

    gtk_widget_set_size_request((*switcher).panel, panel_width, -1);
    gtk_scrolled_window_set_min_content_width(
        (*switcher).scroller as *mut GtkScrolledWindow,
        grid_width,
    );
    gtk_scrolled_window_set_max_content_width(
        (*switcher).scroller as *mut GtkScrolledWindow,
        grid_width,
    );
    gtk_scrolled_window_set_max_content_height(
        (*switcher).scroller as *mut GtkScrolledWindow,
        scroller_height,
    );
    gtk_widget_set_margin_start((*switcher).panel, PANEL_MARGIN);
    gtk_widget_set_margin_end((*switcher).panel, PANEL_MARGIN);
    gtk_widget_set_margin_top((*switcher).panel, top_margin);
}

unsafe fn find_settings_channel(
    switcher: *mut ChannelSwitcherOverlay,
    channel_name: *const c_char,
) -> *const AppSettingsChannel {
    let channel_count = app_settings_get_channel_count((*switcher).settings);

    for i in 0..channel_count {
        let channel = app_settings_get_channel((*switcher).settings, i);
        if !channel.is_null()
            && !(*channel).channel.is_null()
            && !channel_name.is_null()
            && g_ascii_strcasecmp((*channel).channel, channel_name) == 0
        {
            return channel;
        }
    }

    ptr::null()
}

unsafe fn extract_twitch_channel_name(value: *const c_char) -> *mut c_char {
    if value.is_null() {
        return ptr::null_mut();
    }

    let bytes = CStr::from_ptr(value).to_bytes();
    let start_trim = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end_trim = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(start_trim);
    let trimmed = &bytes[start_trim..end_trim];
    if trimmed.is_empty() {
        return ptr::null_mut();
    }

    let twitch_prefix = b"twitch.tv/";
    let mut start = find_bytes(trimmed, twitch_prefix)
        .map(|index| index + twitch_prefix.len())
        .unwrap_or(0);
    while start < trimmed.len() && (trimmed[start] == b'/' || trimmed[start] == b'@') {
        start += 1;
    }

    let mut end = start;
    while end < trimmed.len() && (trimmed[end].is_ascii_alphanumeric() || trimmed[end] == b'_') {
        end += 1;
    }
    if end == start {
        return ptr::null_mut();
    }

    let mut channel = trimmed[start..end].to_vec();
    channel.push(0);
    g_ascii_strdown(channel.as_ptr() as *const c_char, -1)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

unsafe extern "C" fn on_channel_button_clicked(button: *mut GtkButton, user_data: *mut c_void) {
    let switcher = user_data as *mut ChannelSwitcherOverlay;
    let channel_name =
        g_object_get_data(button as *mut GObject, cstr!("channel-name")) as *const c_char;
    let channel = find_settings_channel(switcher, channel_name);

    if !channel.is_null() {
        if let Some(callback) = (*switcher).activate_callback {
            callback(channel, (*switcher).user_data);
        }
    } else if is_nonempty(channel_name) {
        if let Some(callback) = (*switcher).activate_callback {
            let mut dynamic_channel = AppSettingsChannel {
                channel: channel_name as *mut c_char,
                label: g_object_get_data(button as *mut GObject, cstr!("channel-label"))
                    as *mut c_char,
                url: g_strdup_printf(cstr!("https://www.twitch.tv/%s"), channel_name),
            };
            if !is_nonempty(dynamic_channel.label) {
                dynamic_channel.label = dynamic_channel.channel;
            }
            callback(&dynamic_channel, (*switcher).user_data);
            g_free(dynamic_channel.url as *mut c_void);
        }
    }

    channel_switcher_overlay_hide(switcher);
}

unsafe fn activate_dynamic_channel(
    switcher: *mut ChannelSwitcherOverlay,
    channel_name: *const c_char,
) {
    if switcher.is_null() || (*switcher).activate_callback.is_none() || !is_nonempty(channel_name) {
        return;
    }

    let configured_channel = find_settings_channel(switcher, channel_name);
    if !configured_channel.is_null() {
        if let Some(callback) = (*switcher).activate_callback {
            callback(configured_channel, (*switcher).user_data);
        }
        channel_switcher_overlay_hide(switcher);
        return;
    }

    let dynamic_channel = AppSettingsChannel {
        channel: channel_name as *mut c_char,
        label: channel_name as *mut c_char,
        url: g_strdup_printf(cstr!("https://www.twitch.tv/%s"), channel_name),
    };

    if let Some(callback) = (*switcher).activate_callback {
        callback(&dynamic_channel, (*switcher).user_data);
    }
    g_free(dynamic_channel.url as *mut c_void);
    channel_switcher_overlay_hide(switcher);
}

unsafe fn open_direct_channel(switcher: *mut ChannelSwitcherOverlay) {
    if switcher.is_null() || (*switcher).direct_channel_entry.is_null() {
        return;
    }

    let text = gtk_editable_get_text((*switcher).direct_channel_entry as *mut GtkEditable);
    let channel = extract_twitch_channel_name(text);
    activate_dynamic_channel(switcher, channel);
    g_free(channel as *mut c_void);
}

unsafe extern "C" fn on_direct_channel_activate(_entry: *mut GtkEntry, user_data: *mut c_void) {
    open_direct_channel(user_data as *mut ChannelSwitcherOverlay);
}

unsafe extern "C" fn on_direct_channel_icon_pressed(
    _entry: *mut GtkEntry,
    icon_pos: c_int,
    user_data: *mut c_void,
) {
    if icon_pos == GTK_ENTRY_ICON_SECONDARY {
        open_direct_channel(user_data as *mut ChannelSwitcherOverlay);
    }
}

unsafe fn create_image_picture(
    switcher: *mut ChannelSwitcherOverlay,
    url: *const c_char,
    width: c_int,
    height: c_int,
    css_class: *const c_char,
) -> *mut GtkWidget {
    let image = gtk_picture_new();
    let placeholder = create_placeholder_texture(width, height);

    gtk_widget_add_css_class(image, css_class);
    gtk_widget_set_focusable(image, FALSE);
    gtk_widget_set_size_request(image, width, height);
    gtk_widget_set_halign(image, GTK_ALIGN_START);
    gtk_widget_set_valign(image, GTK_ALIGN_START);
    gtk_widget_set_hexpand(image, FALSE);
    gtk_widget_set_vexpand(image, FALSE);
    gtk_widget_set_overflow(image, GTK_OVERFLOW_HIDDEN);
    gtk_picture_set_content_fit(image as *mut GtkPicture, GTK_CONTENT_FIT_COVER);
    gtk_picture_set_can_shrink(image as *mut GtkPicture, TRUE);
    if !placeholder.is_null() {
        gtk_picture_set_paintable(image as *mut GtkPicture, placeholder as *mut GdkPaintable);
        g_object_unref(placeholder as *mut c_void);
    }
    load_remote_image(switcher, image, url, width, height);
    image
}

unsafe fn create_fixed_picture_frame(
    picture: *mut GtkWidget,
    width: c_int,
    height: c_int,
) -> *mut GtkWidget {
    let frame = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);

    gtk_widget_add_css_class(frame, cstr!("channel-switcher-image-frame"));
    gtk_widget_set_size_request(frame, width, height);
    gtk_widget_set_halign(frame, GTK_ALIGN_START);
    gtk_widget_set_valign(frame, GTK_ALIGN_START);
    gtk_widget_set_hexpand(frame, FALSE);
    gtk_widget_set_vexpand(frame, FALSE);
    gtk_widget_set_overflow(frame, GTK_OVERFLOW_HIDDEN);
    gtk_box_append(frame as *mut GtkBox, picture);

    frame
}

unsafe fn format_viewer_count(viewer_count: c_uint) -> *mut c_char {
    let count = twitch_stream_info_format_viewer_count(viewer_count);
    let result = g_strdup_printf(cstr!("%s viewers"), count);
    g_free(count as *mut c_void);
    result
}

unsafe fn format_meta_text(preview: *mut TwitchStreamPreview) -> *mut c_char {
    let viewers = format_viewer_count((*preview).viewer_count);
    let duration = twitch_stream_info_format_live_duration((*preview).started_at);
    let result = g_strdup_printf(
        cstr!("%s \u{2022} %s"),
        viewers,
        if duration.is_null() {
            cstr!("live")
        } else {
            duration
        },
    );
    g_free(viewers as *mut c_void);
    g_free(duration as *mut c_void);
    result
}

unsafe fn create_channel_card(
    switcher: *mut ChannelSwitcherOverlay,
    preview: *mut TwitchStreamPreview,
    card_width: c_int,
    preview_width: c_int,
    preview_height: c_int,
) -> *mut GtkWidget {
    let channel = find_settings_channel(switcher, (*preview).channel);
    let label = if !channel.is_null() && is_nonempty((*channel).label) {
        (*channel).label
    } else {
        (*preview).display_name
    };

    let button = gtk_button_new();
    gtk_widget_add_css_class(button, cstr!("channel-switcher-item"));
    gtk_widget_set_halign(button, GTK_ALIGN_START);
    gtk_widget_set_hexpand(button, FALSE);
    gtk_widget_set_size_request(button, card_width, -1);
    g_object_set_data_full(
        button as *mut GObject,
        cstr!("channel-name"),
        g_strdup((*preview).channel) as *mut c_void,
        Some(g_free_destroy),
    );
    g_object_set_data_full(
        button as *mut GObject,
        cstr!("channel-label"),
        g_strdup(label) as *mut c_void,
        Some(g_free_destroy),
    );
    g_signal_connect_data(
        button as *mut c_void,
        cstr!("clicked"),
        on_channel_button_clicked as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );

    let card = gtk_box_new(GTK_ORIENTATION_VERTICAL, 5);
    gtk_widget_set_halign(card, GTK_ALIGN_START);
    gtk_widget_set_hexpand(card, FALSE);
    gtk_widget_set_size_request(card, card_width, -1);

    let preview_frame = create_fixed_picture_frame(
        create_image_picture(
            switcher,
            (*preview).preview_url,
            preview_width,
            preview_height,
            cstr!("channel-switcher-preview"),
        ),
        preview_width,
        preview_height,
    );
    gtk_widget_set_halign(preview_frame, GTK_ALIGN_CENTER);

    let text_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 4);
    gtk_widget_set_hexpand(text_box, TRUE);
    gtk_widget_set_valign(text_box, GTK_ALIGN_START);

    let details_row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 7);
    gtk_widget_set_halign(details_row, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(details_row, TRUE);

    let avatar_frame = create_fixed_picture_frame(
        create_image_picture(
            switcher,
            (*preview).avatar_url,
            AVATAR_SIZE,
            AVATAR_SIZE,
            cstr!("channel-switcher-avatar"),
        ),
        AVATAR_SIZE,
        AVATAR_SIZE,
    );

    let title_label = gtk_label_new(if (*preview).title.is_null() {
        cstr!("")
    } else {
        (*preview).title
    });
    gtk_widget_add_css_class(title_label, cstr!("channel-switcher-title"));
    gtk_label_set_xalign(title_label as *mut GtkLabel, 0.0);
    gtk_label_set_single_line_mode(title_label as *mut GtkLabel, TRUE);
    gtk_label_set_ellipsize(title_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
    gtk_label_set_max_width_chars(title_label as *mut GtkLabel, 26);
    gtk_widget_set_halign(title_label, GTK_ALIGN_FILL);

    let name_label = gtk_label_new(label);
    gtk_widget_add_css_class(name_label, cstr!("channel-switcher-name"));
    gtk_label_set_xalign(name_label as *mut GtkLabel, 0.0);
    gtk_label_set_ellipsize(name_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
    gtk_label_set_max_width_chars(name_label as *mut GtkLabel, 26);
    gtk_widget_set_halign(name_label, GTK_ALIGN_FILL);

    let meta_text = format_meta_text(preview);
    let meta_label = gtk_label_new(meta_text);
    gtk_widget_add_css_class(meta_label, cstr!("channel-switcher-meta"));
    gtk_label_set_xalign(meta_label as *mut GtkLabel, 0.0);
    gtk_label_set_ellipsize(meta_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
    gtk_label_set_max_width_chars(meta_label as *mut GtkLabel, 26);
    gtk_widget_set_halign(meta_label, GTK_ALIGN_FILL);
    g_free(meta_text as *mut c_void);

    gtk_box_append(text_box as *mut GtkBox, title_label);
    gtk_box_append(text_box as *mut GtkBox, name_label);
    gtk_box_append(text_box as *mut GtkBox, meta_label);
    if is_nonempty((*preview).category_name) {
        let category_label = gtk_label_new((*preview).category_name);
        gtk_widget_add_css_class(category_label, cstr!("channel-switcher-meta"));
        gtk_label_set_xalign(category_label as *mut GtkLabel, 0.0);
        gtk_label_set_ellipsize(category_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
        gtk_label_set_max_width_chars(category_label as *mut GtkLabel, 26);
        gtk_widget_set_halign(category_label, GTK_ALIGN_FILL);
        gtk_box_append(text_box as *mut GtkBox, category_label);
    }
    gtk_box_append(details_row as *mut GtkBox, avatar_frame);
    gtk_box_append(details_row as *mut GtkBox, text_box);
    gtk_box_append(card as *mut GtkBox, preview_frame);
    gtk_box_append(card as *mut GtkBox, details_row);
    gtk_button_set_child(button as *mut GtkButton, card);

    button
}

unsafe fn string_contains_casefold(haystack: *const c_char, needle: *const c_char) -> bool {
    if !is_nonempty(needle) {
        return true;
    }
    if !is_nonempty(haystack) {
        return false;
    }

    let haystack_folded = g_utf8_casefold(haystack, -1);
    let needle_folded = g_utf8_casefold(needle, -1);
    let found = if haystack_folded.is_null() || needle_folded.is_null() {
        false
    } else {
        let haystack_bytes = CStr::from_ptr(haystack_folded).to_bytes();
        let needle_bytes = CStr::from_ptr(needle_folded).to_bytes();
        find_bytes(haystack_bytes, needle_bytes).is_some()
    };
    g_free(haystack_folded as *mut c_void);
    g_free(needle_folded as *mut c_void);
    found
}

unsafe fn preview_matches_filter(preview: *mut TwitchStreamPreview, filter: *const c_char) -> bool {
    string_contains_casefold((*preview).channel, filter)
        || string_contains_casefold((*preview).display_name, filter)
        || string_contains_casefold((*preview).title, filter)
        || string_contains_casefold((*preview).category_name, filter)
}

unsafe fn ensure_preview_cards(switcher: *mut ChannelSwitcherOverlay) {
    let mut columns = 1;
    let mut card_width = CARD_WIDTH;
    let mut preview_width = PREVIEW_WIDTH;
    let mut preview_height = PREVIEW_HEIGHT;

    calculate_card_layout(
        switcher,
        &mut columns,
        &mut card_width,
        &mut preview_width,
        &mut preview_height,
    );

    if (*switcher).preview_cards.is_null() {
        (*switcher).preview_cards = g_ptr_array_new_with_free_func(Some(g_object_unref_destroy));
    }

    if (*switcher).previews.is_null() {
        return;
    }

    if (*(*switcher).preview_cards).len == (*(*switcher).previews).len
        && (*switcher).preview_card_columns == columns
        && (*switcher).preview_card_width == card_width
        && (*switcher).preview_width == preview_width
        && (*switcher).preview_height == preview_height
    {
        return;
    }

    clear_preview_cards(switcher);
    (*switcher).preview_card_columns = columns;
    (*switcher).preview_card_width = card_width;
    (*switcher).preview_width = preview_width;
    (*switcher).preview_height = preview_height;
    for i in 0..(*(*switcher).previews).len {
        let preview = ptr_array_index((*switcher).previews, i);
        let card =
            create_channel_card(switcher, preview, card_width, preview_width, preview_height);
        g_ptr_array_add(
            (*switcher).preview_cards,
            g_object_ref_sink(card as *mut c_void),
        );
    }
}

unsafe fn render_live_channels(switcher: *mut ChannelSwitcherOverlay) {
    if (*switcher).previews.is_null() || (*(*switcher).previews).len == 0 {
        show_status(switcher, cstr!("No configured channels are live"));
        return;
    }

    ensure_preview_cards(switcher);

    let filter = if !(*switcher).search_entry.is_null() {
        gtk_editable_get_text((*switcher).search_entry as *mut GtkEditable)
    } else {
        cstr!("")
    };
    let mut visible_count = 0;

    clear_grid(switcher);
    let columns = if (*switcher).preview_card_columns > 0 {
        (*switcher).preview_card_columns
    } else {
        get_grid_columns(switcher)
    };
    let preview_count = (*(*switcher).previews)
        .len
        .min((*(*switcher).preview_cards).len);
    for i in 0..preview_count {
        let preview = ptr_array_index((*switcher).previews, i);
        if !preview_matches_filter(preview, filter) {
            continue;
        }

        gtk_grid_attach(
            (*switcher).grid as *mut GtkGrid,
            ptr_array_index((*switcher).preview_cards, i),
            (visible_count % columns) as c_int,
            (visible_count / columns) as c_int,
            1,
            1,
        );
        visible_count += 1;
    }

    if visible_count == 0 {
        show_status(switcher, cstr!("No live channels match the filter"));
    }
}

unsafe extern "C" fn apply_search_filter(user_data: *mut c_void) -> c_int {
    let switcher = user_data as *mut ChannelSwitcherOverlay;

    (*switcher).search_debounce_source = 0;
    render_live_channels(switcher);

    G_SOURCE_REMOVE
}

unsafe extern "C" fn on_search_changed(_editable: *mut GtkEditable, user_data: *mut c_void) {
    let switcher = user_data as *mut ChannelSwitcherOverlay;

    remove_source_if_active(&mut (*switcher).search_debounce_source);
    if (*switcher).panel.is_null()
        || gtk_widget_get_visible((*switcher).panel) == 0
        || (*switcher).previews.is_null()
    {
        return;
    }

    (*switcher).search_debounce_source = g_timeout_add(
        SEARCH_DEBOUNCE_MS,
        Some(apply_search_filter),
        switcher as *mut c_void,
    );
}

unsafe fn activate_first_visible_channel(switcher: *mut ChannelSwitcherOverlay) {
    if switcher.is_null() || (*switcher).grid.is_null() {
        return;
    }

    let button_type = gtk_button_get_type();
    let mut child = gtk_widget_get_first_child((*switcher).grid);
    while !child.is_null() {
        if g_type_check_instance_is_a(child as *mut GTypeInstance, button_type) != 0 {
            g_signal_emit_by_name(child as *mut c_void, cstr!("clicked"));
            return;
        }
        child = gtk_widget_get_next_sibling(child);
    }
}

unsafe extern "C" fn on_search_activate(_entry: *mut GtkSearchEntry, user_data: *mut c_void) {
    let switcher = user_data as *mut ChannelSwitcherOverlay;

    remove_source_if_active(&mut (*switcher).search_debounce_source);
    render_live_channels(switcher);
    activate_first_visible_channel(switcher);
}

unsafe extern "C" fn on_live_channels_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let data = user_data as *mut LiveFetchCallbackData;
    let switcher = (*data).switcher;
    let mut error: *mut GError = ptr::null_mut();
    let previews = twitch_stream_info_fetch_live_channels_finish(result, &mut error);

    if (*data).generation != (*switcher).generation || (*switcher).panel.is_null() {
        g_clear_error(&mut error);
        if !previews.is_null() {
            g_ptr_array_unref(previews);
        }
        live_fetch_callback_data_free(data);
        return;
    }

    clear_object(&mut (*switcher).cancel);

    if !error.is_null() {
        if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
            g_log(
                G_LOG_DOMAIN.as_ptr() as *const c_char,
                G_LOG_LEVEL_DEBUG,
                cstr!("live channel fetch failed: %s"),
                (*error).message,
            );
            show_status(switcher, cstr!("Live channels could not be loaded"));
        }
        g_clear_error(&mut error);
        if !previews.is_null() {
            g_ptr_array_unref(previews);
        }
        live_fetch_callback_data_free(data);
        return;
    }

    if !(*switcher).previews.is_null() {
        g_ptr_array_unref((*switcher).previews);
    }
    clear_preview_cards(switcher);
    (*switcher).previews = if previews.is_null() {
        ptr::null_mut()
    } else {
        g_ptr_array_ref(previews)
    };
    if !previews.is_null() {
        g_ptr_array_unref(previews);
    }
    (*switcher).cached_at_us = g_get_monotonic_time();
    render_live_channels(switcher);

    live_fetch_callback_data_free(data);
}

unsafe fn build_channels_cache_key(
    channels: *mut *mut c_char,
    channel_count: c_uint,
) -> *mut c_char {
    let mut key = Vec::new();

    for i in 0..channel_count {
        if i > 0 {
            key.push(b'\n');
        }
        let channel = *channels.add(i as usize);
        if !channel.is_null() {
            key.extend_from_slice(CStr::from_ptr(channel).to_bytes());
        }
    }

    dup_bytes(&key)
}

unsafe fn has_fresh_cache(
    switcher: *mut ChannelSwitcherOverlay,
    channels_key: *const c_char,
) -> bool {
    let now_us = g_get_monotonic_time();

    !(*switcher).previews.is_null()
        && !(*switcher).cached_channels_key.is_null()
        && !channels_key.is_null()
        && g_strcmp0((*switcher).cached_channels_key, channels_key) == 0
        && now_us - (*switcher).cached_at_us < LIVE_CHANNELS_CACHE_SECONDS * G_USEC_PER_SEC
}

unsafe fn start_live_channel_fetch(
    switcher: *mut ChannelSwitcherOverlay,
    channels: *mut *mut c_char,
    channel_count: c_uint,
    allow_cache: c_int,
) {
    let channels_key = build_channels_cache_key(channels, channel_count);

    if channel_count == 0 {
        if !(*switcher).previews.is_null() {
            g_ptr_array_unref((*switcher).previews);
            (*switcher).previews = ptr::null_mut();
        }
        clear_preview_cards(switcher);
        clear_image_cache(switcher);
        g_free((*switcher).cached_channels_key as *mut c_void);
        (*switcher).cached_channels_key = ptr::null_mut();
        (*switcher).cached_at_us = 0;
        show_status(switcher, cstr!("No channels configured"));
        g_free(channels_key as *mut c_void);
        return;
    }

    if allow_cache != 0 && has_fresh_cache(switcher, channels_key) {
        render_live_channels(switcher);
        g_free(channels_key as *mut c_void);
        return;
    }

    if !(*switcher).previews.is_null() {
        g_ptr_array_unref((*switcher).previews);
        (*switcher).previews = ptr::null_mut();
    }
    clear_preview_cards(switcher);
    clear_image_cache(switcher);
    g_free((*switcher).cached_channels_key as *mut c_void);
    (*switcher).cached_channels_key = g_strdup(channels_key);
    (*switcher).cached_at_us = 0;

    (*switcher).cancel = g_cancellable_new();
    let data = Box::into_raw(Box::new(LiveFetchCallbackData {
        switcher,
        generation: (*switcher).generation,
    }));
    twitch_stream_info_fetch_live_channels_async(
        channels as *const *const c_char,
        channel_count,
        (*switcher).cancel,
        Some(on_live_channels_fetched),
        data as *mut c_void,
    );
    g_free(channels_key as *mut c_void);
}

unsafe extern "C" fn on_channel_list_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let data = user_data as *mut ChannelListFetchCallbackData;
    let switcher = (*data).switcher;
    let mut error: *mut GError = ptr::null_mut();
    let mut channel_count = 0;
    let channels = twitch_channel_list_fetch_finish(result, &mut channel_count, &mut error);

    if (*data).generation != (*switcher).generation || (*switcher).panel.is_null() {
        g_clear_error(&mut error);
        g_strfreev(channels);
        drop(Box::from_raw(data));
        return;
    }

    clear_object(&mut (*switcher).cancel);

    if !error.is_null() {
        if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
            log_debug(
                cstr!("channel list fetch failed: %s%s"),
                (*error).message,
                cstr!(""),
            );
            show_status(switcher, (*error).message);
        }
        g_clear_error(&mut error);
        g_strfreev(channels);
        drop(Box::from_raw(data));
        return;
    }

    start_live_channel_fetch(switcher, channels, channel_count, TRUE);

    g_strfreev(channels);
    drop(Box::from_raw(data));
}

unsafe extern "C" fn on_close_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    channel_switcher_overlay_hide(user_data as *mut ChannelSwitcherOverlay);
}

unsafe extern "C" fn on_settings_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let switcher = user_data as *mut ChannelSwitcherOverlay;
    let callback = (*switcher).settings_callback;
    let callback_data = (*switcher).settings_user_data;

    channel_switcher_overlay_hide(switcher);
    if let Some(callback) = callback {
        callback(callback_data);
    }
}

unsafe extern "C" fn on_backdrop_pressed(
    _gesture: *mut GtkGestureClick,
    n_press: c_int,
    _x: c_double,
    _y: c_double,
    user_data: *mut c_void,
) {
    if n_press == 1 {
        channel_switcher_overlay_hide(user_data as *mut ChannelSwitcherOverlay);
    }
}

unsafe fn add_weak_pointer<T>(object: *mut T, slot: *mut *mut T) {
    g_object_add_weak_pointer(object as *mut GObject, slot as *mut *mut c_void);
}

pub unsafe fn channel_switcher_overlay_new<O>(
    overlay: *mut O,
    settings: *mut AppSettings,
    activate_callback: ChannelSwitcherActivateCallback,
    user_data: *mut c_void,
    settings_callback: ChannelSwitcherSettingsCallback,
    settings_user_data: *mut c_void,
) -> *mut ChannelSwitcherOverlay {
    let overlay = overlay as *mut GtkOverlay;
    install_css();

    let switcher = Box::into_raw(Box::new(ChannelSwitcherOverlay {
        overlay,
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
    }));
    add_weak_pointer((*switcher).overlay, &mut (*switcher).overlay);

    (*switcher).backdrop = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    add_weak_pointer((*switcher).backdrop, &mut (*switcher).backdrop);
    gtk_widget_add_css_class((*switcher).backdrop, cstr!("channel-switcher-backdrop"));
    gtk_widget_set_halign((*switcher).backdrop, GTK_ALIGN_FILL);
    gtk_widget_set_valign((*switcher).backdrop, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand((*switcher).backdrop, TRUE);
    gtk_widget_set_vexpand((*switcher).backdrop, TRUE);
    gtk_widget_set_visible((*switcher).backdrop, FALSE);
    let backdrop_click = gtk_gesture_click_new();
    gtk_gesture_single_set_button(backdrop_click as *mut GtkGestureSingle, GDK_BUTTON_PRIMARY);
    g_signal_connect_data(
        backdrop_click as *mut c_void,
        cstr!("pressed"),
        on_backdrop_pressed as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*switcher).backdrop, backdrop_click as *mut c_void);
    gtk_overlay_add_overlay(overlay, (*switcher).backdrop);

    (*switcher).panel = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    add_weak_pointer((*switcher).panel, &mut (*switcher).panel);
    gtk_widget_add_css_class((*switcher).panel, cstr!("channel-switcher-panel"));
    gtk_widget_set_halign((*switcher).panel, GTK_ALIGN_CENTER);
    gtk_widget_set_valign((*switcher).panel, GTK_ALIGN_START);
    gtk_widget_set_hexpand((*switcher).panel, FALSE);
    gtk_widget_set_visible((*switcher).panel, FALSE);

    let header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    gtk_widget_add_css_class(header, cstr!("channel-switcher-header"));
    (*switcher).search_entry = gtk_search_entry_new();
    add_weak_pointer((*switcher).search_entry, &mut (*switcher).search_entry);
    gtk_widget_add_css_class((*switcher).search_entry, cstr!("channel-switcher-search"));
    gtk_search_entry_set_placeholder_text(
        (*switcher).search_entry as *mut GtkSearchEntry,
        cstr!("Filter live channels"),
    );
    g_signal_connect_data(
        (*switcher).search_entry as *mut c_void,
        cstr!("changed"),
        on_search_changed as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*switcher).search_entry as *mut c_void,
        cstr!("activate"),
        on_search_activate as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );
    (*switcher).direct_channel_entry = gtk_entry_new();
    add_weak_pointer(
        (*switcher).direct_channel_entry,
        &mut (*switcher).direct_channel_entry,
    );
    gtk_widget_add_css_class(
        (*switcher).direct_channel_entry,
        cstr!("channel-switcher-open-entry"),
    );
    gtk_entry_set_placeholder_text(
        (*switcher).direct_channel_entry as *mut GtkEntry,
        cstr!("Channel or Twitch URL"),
    );
    gtk_entry_set_icon_from_icon_name(
        (*switcher).direct_channel_entry as *mut GtkEntry,
        GTK_ENTRY_ICON_SECONDARY,
        cstr!("media-playback-start-symbolic"),
    );
    gtk_entry_set_icon_tooltip_text(
        (*switcher).direct_channel_entry as *mut GtkEntry,
        GTK_ENTRY_ICON_SECONDARY,
        cstr!("Open channel"),
    );
    gtk_widget_set_tooltip_text(
        (*switcher).direct_channel_entry,
        cstr!("Enter a channel name or Twitch URL"),
    );
    g_signal_connect_data(
        (*switcher).direct_channel_entry as *mut c_void,
        cstr!("activate"),
        on_direct_channel_activate as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );
    g_signal_connect_data(
        (*switcher).direct_channel_entry as *mut c_void,
        cstr!("icon-press"),
        on_direct_channel_icon_pressed as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );
    let header_spacer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_set_hexpand(header_spacer, TRUE);
    let settings_button = gtk_button_new();
    gtk_button_set_child(
        settings_button as *mut GtkButton,
        player_settings_icon_new(),
    );
    gtk_widget_add_css_class(settings_button, cstr!("channel-switcher-action"));
    gtk_widget_set_tooltip_text(settings_button, cstr!("Edit channels"));
    g_signal_connect_data(
        settings_button as *mut c_void,
        cstr!("clicked"),
        on_settings_clicked as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );
    let close_button = gtk_button_new();
    gtk_button_set_child(
        close_button as *mut GtkButton,
        player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE),
    );
    gtk_widget_add_css_class(close_button, cstr!("channel-switcher-close"));
    gtk_widget_set_tooltip_text(close_button, cstr!("Close"));
    g_signal_connect_data(
        close_button as *mut c_void,
        cstr!("clicked"),
        on_close_clicked as *const c_void,
        switcher as *mut c_void,
        ptr::null_mut(),
        0,
    );
    let input_separator = gtk_label_new(cstr!("|"));
    gtk_widget_add_css_class(input_separator, cstr!("channel-switcher-header-separator"));
    gtk_box_append(header as *mut GtkBox, (*switcher).search_entry);
    gtk_box_append(header as *mut GtkBox, input_separator);
    gtk_box_append(header as *mut GtkBox, (*switcher).direct_channel_entry);
    gtk_box_append(header as *mut GtkBox, header_spacer);
    gtk_box_append(header as *mut GtkBox, settings_button);
    gtk_box_append(header as *mut GtkBox, close_button);
    gtk_box_append((*switcher).panel as *mut GtkBox, header);

    (*switcher).scroller = gtk_scrolled_window_new();
    add_weak_pointer((*switcher).scroller, &mut (*switcher).scroller);
    gtk_widget_add_css_class((*switcher).scroller, cstr!("channel-switcher-scroller"));
    gtk_scrolled_window_set_policy(
        (*switcher).scroller as *mut GtkScrolledWindow,
        GTK_POLICY_NEVER,
        GTK_POLICY_AUTOMATIC,
    );
    gtk_scrolled_window_set_propagate_natural_width(
        (*switcher).scroller as *mut GtkScrolledWindow,
        TRUE,
    );
    gtk_scrolled_window_set_propagate_natural_height(
        (*switcher).scroller as *mut GtkScrolledWindow,
        TRUE,
    );

    (*switcher).grid = gtk_grid_new();
    add_weak_pointer((*switcher).grid, &mut (*switcher).grid);
    gtk_widget_add_css_class((*switcher).grid, cstr!("channel-switcher-grid"));
    gtk_widget_set_halign((*switcher).grid, GTK_ALIGN_START);
    gtk_widget_set_hexpand((*switcher).grid, FALSE);
    gtk_grid_set_column_spacing((*switcher).grid as *mut GtkGrid, CARD_SPACING as c_uint);
    gtk_grid_set_row_spacing((*switcher).grid as *mut GtkGrid, CARD_SPACING as c_uint);
    gtk_scrolled_window_set_child(
        (*switcher).scroller as *mut GtkScrolledWindow,
        (*switcher).grid,
    );
    gtk_box_append((*switcher).panel as *mut GtkBox, (*switcher).scroller);
    gtk_overlay_add_overlay(overlay, (*switcher).panel);

    switcher
}

pub unsafe fn channel_switcher_overlay_set_settings(
    switcher: *mut ChannelSwitcherOverlay,
    settings: *mut AppSettings,
) {
    if switcher.is_null() {
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

pub unsafe fn channel_switcher_overlay_show_at(
    switcher: *mut ChannelSwitcherOverlay,
    _x: c_double,
    _y: c_double,
) {
    if switcher.is_null() || (*switcher).settings.is_null() || (*switcher).panel.is_null() {
        return;
    }

    bump_generation(switcher);
    remove_source_if_active(&mut (*switcher).search_debounce_source);
    if !(*switcher).search_entry.is_null() {
        gtk_editable_set_text((*switcher).search_entry as *mut GtkEditable, cstr!(""));
    }
    if !(*switcher).direct_channel_entry.is_null() {
        gtk_editable_set_text(
            (*switcher).direct_channel_entry as *mut GtkEditable,
            cstr!(""),
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
    show_status(switcher, cstr!("Loading live channels"));

    clear_object(&mut (*switcher).cancel);
    (*switcher).cancel = g_cancellable_new();
    let data = Box::into_raw(Box::new(ChannelListFetchCallbackData {
        switcher,
        generation: (*switcher).generation,
    }));
    twitch_channel_list_fetch_async(
        (*switcher).settings,
        (*switcher).cancel,
        Some(on_channel_list_fetched),
        data as *mut c_void,
    );
}

pub unsafe fn channel_switcher_overlay_hide(switcher: *mut ChannelSwitcherOverlay) {
    if switcher.is_null() {
        return;
    }

    bump_generation(switcher);
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
            cstr!(""),
        );
    }
    clear_grid(switcher);
    clear_image_cache(switcher);
}

pub unsafe fn channel_switcher_overlay_is_visible(switcher: *mut ChannelSwitcherOverlay) -> c_int {
    (switcher.is_null() == false
        && !(*switcher).panel.is_null()
        && gtk_widget_get_visible((*switcher).panel) != 0) as c_int
}

pub unsafe fn channel_switcher_overlay_free(switcher: *mut ChannelSwitcherOverlay) {
    if switcher.is_null() {
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
    (*switcher).panel = ptr::null_mut();
    (*switcher).backdrop = ptr::null_mut();
    (*switcher).grid = ptr::null_mut();
    (*switcher).scroller = ptr::null_mut();
    (*switcher).search_entry = ptr::null_mut();
    (*switcher).direct_channel_entry = ptr::null_mut();
    (*switcher).overlay = ptr::null_mut();
}
