use std::ffi::{c_char, c_double, c_int, c_uint, c_void, CStr};
use std::ptr;

use crate::player_icons::player_trash_icon_new;
use crate::settings::{
    app_settings_add_channel, app_settings_clear_channels, app_settings_get_channel,
    app_settings_get_channel_count, app_settings_get_hwdec_enabled,
    app_settings_get_twitch_oauth_token, app_settings_get_twitch_refresh_token, app_settings_save,
    app_settings_set_hwdec_enabled, app_settings_set_twitch_auth_tokens,
    app_settings_set_twitch_oauth_token, AppSettings,
};
use crate::twitch_auth::{
    twitch_auth_device_code_free, twitch_auth_poll_device_token_async,
    twitch_auth_poll_device_token_finish, twitch_auth_request_device_code_async,
    twitch_auth_request_device_code_finish, twitch_auth_token_free,
};
use crate::twitch_channel_list::twitch_channel_list_invalidate_followed_cache;
use crate::twitch_stream_info::{GAsyncResult, GCancellable, GError};

const G_IO_ERROR_CANCELLED: c_int = 19;
const G_PRIORITY_DEFAULT_IDLE: c_int = 200;
const G_SOURCE_REMOVE: c_int = 0;
const GTK_ALIGN_FILL: c_int = 0;
const GTK_ALIGN_CENTER: c_int = 3;
const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_POLICY_AUTOMATIC: c_int = 1;
const GTK_POLICY_NEVER: c_int = 2;
const GTK_SELECTION_SINGLE: c_int = 1;
const SETTINGS_WINDOW_PAGE_CHANNELS: c_int = 1;
const TWITCH_AUTH_CLIENT_ID: &[u8] = b"8l1fzyh4jhs1cxhtqs6p4swmxuejh6\0";

pub struct SettingsWindow {
    window: *mut GtkWidget,
    sidebar: *mut GtkWidget,
    stack: *mut GtkWidget,
    hwdec_check: *mut GtkWidget,
    twitch_auth_button: *mut GtkWidget,
    twitch_auth_status: *mut GtkWidget,
    channels_box: *mut GtkWidget,
    empty_label: *mut GtkWidget,
    status_label: *mut GtkWidget,
    settings: *mut AppSettings,
    saved_callback: Option<SettingsWindowSavedCallback>,
    user_data: *mut c_void,
    auth_cancel: *mut GCancellable,
    auth_in_progress: c_int,
}

pub struct ChannelRow {
    view: *mut SettingsWindow,
    row: *mut GtkWidget,
    channel_entry: *mut GtkWidget,
}

#[repr(C)]
pub struct GObject {
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
pub struct GtkCheckButton {
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
pub struct GtkLabel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkListBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkListBoxRow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkScrolledWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkStack {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkUriLauncher {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

type GAsyncReadyCallback = unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void);
type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type SettingsWindowSavedCallback = unsafe extern "C" fn(*mut AppSettings, *mut c_void);

unsafe extern "C" {
    fn g_cancellable_cancel(cancellable: *mut GCancellable);
    fn g_cancellable_new() -> *mut GCancellable;
    fn g_clear_error(error: *mut *mut GError);
    fn g_error_matches(error: *mut GError, domain: c_uint, code: c_int) -> c_int;
    fn g_free(mem: *mut c_void);
    fn g_get_real_time() -> i64;
    fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    fn g_io_error_quark() -> c_uint;
    fn g_markup_escape_text(text: *const c_char, length: isize) -> *mut c_char;
    fn g_object_get_data(object: *mut GObject, key: *const c_char) -> *mut c_void;
    fn g_object_ref(object: *mut c_void) -> *mut c_void;
    fn g_object_set_data_full(
        object: *mut GObject,
        key: *const c_char,
        data: *mut c_void,
        destroy: Option<GDestroyNotify>,
    );
    fn g_object_unref(object: *mut c_void);
    fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    fn g_strdup(str: *const c_char) -> *mut c_char;

    fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    fn gtk_box_remove(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_button_new() -> *mut GtkWidget;
    fn gtk_button_new_with_label(label: *const c_char) -> *mut GtkWidget;
    fn gtk_button_set_child(button: *mut GtkButton, child: *mut GtkWidget);
    fn gtk_button_set_has_frame(button: *mut GtkButton, has_frame: c_int);
    fn gtk_button_set_label(button: *mut GtkButton, label: *const c_char);
    fn gtk_check_button_get_active(check_button: *mut GtkCheckButton) -> c_int;
    fn gtk_check_button_new_with_label(label: *const c_char) -> *mut GtkWidget;
    fn gtk_check_button_set_active(check_button: *mut GtkCheckButton, setting: c_int);
    fn gtk_editable_get_text(editable: *mut GtkEditable) -> *const c_char;
    fn gtk_editable_select_region(editable: *mut GtkEditable, start_pos: c_int, end_pos: c_int);
    fn gtk_editable_set_text(editable: *mut GtkEditable, text: *const c_char);
    fn gtk_entry_new() -> *mut GtkWidget;
    fn gtk_entry_set_placeholder_text(entry: *mut GtkEntry, text: *const c_char);
    fn gtk_label_new(str: *const c_char) -> *mut GtkWidget;
    fn gtk_label_set_markup(label: *mut GtkLabel, str: *const c_char);
    fn gtk_label_set_text(label: *mut GtkLabel, str: *const c_char);
    fn gtk_label_set_use_markup(label: *mut GtkLabel, setting: c_int);
    fn gtk_label_set_wrap(label: *mut GtkLabel, wrap: c_int);
    fn gtk_label_set_xalign(label: *mut GtkLabel, xalign: c_double);
    fn gtk_list_box_append(box_: *mut GtkListBox, child: *mut GtkWidget);
    fn gtk_list_box_new() -> *mut GtkWidget;
    fn gtk_list_box_row_new() -> *mut GtkWidget;
    fn gtk_list_box_row_set_child(row: *mut GtkListBoxRow, child: *mut GtkWidget);
    fn gtk_list_box_select_row(box_: *mut GtkListBox, row: *mut GtkListBoxRow);
    fn gtk_list_box_set_selection_mode(box_: *mut GtkListBox, mode: c_int);
    fn gtk_scrolled_window_new() -> *mut GtkWidget;
    fn gtk_scrolled_window_set_child(
        scrolled_window: *mut GtkScrolledWindow,
        child: *mut GtkWidget,
    );
    fn gtk_scrolled_window_set_policy(
        scrolled_window: *mut GtkScrolledWindow,
        hscrollbar_policy: c_int,
        vscrollbar_policy: c_int,
    );
    fn gtk_separator_new(orientation: c_int) -> *mut GtkWidget;
    fn gtk_stack_add_named(
        stack: *mut GtkStack,
        child: *mut GtkWidget,
        name: *const c_char,
    ) -> *mut c_void;
    fn gtk_stack_new() -> *mut GtkWidget;
    fn gtk_stack_set_visible_child_name(stack: *mut GtkStack, name: *const c_char);
    fn gtk_uri_launcher_launch(
        self_: *mut GtkUriLauncher,
        parent: *mut GtkWindow,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        user_data: *mut c_void,
    );
    fn gtk_uri_launcher_new(uri: *const c_char) -> *mut GtkUriLauncher;
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_get_first_child(widget: *mut GtkWidget) -> *mut GtkWidget;
    fn gtk_widget_get_next_sibling(widget: *mut GtkWidget) -> *mut GtkWidget;
    fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_sensitive(widget: *mut GtkWidget, sensitive: c_int);
    fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);
    fn gtk_window_close(window: *mut GtkWindow);
    fn gtk_window_new() -> *mut GtkWidget;
    fn gtk_window_present(window: *mut GtkWindow);
    fn gtk_window_set_child(window: *mut GtkWindow, child: *mut GtkWidget);
    fn gtk_window_set_default_size(window: *mut GtkWindow, width: c_int, height: c_int);
    fn gtk_window_set_focus(window: *mut GtkWindow, focus: *mut GtkWidget);
    fn gtk_window_set_modal(window: *mut GtkWindow, modal: c_int);
    fn gtk_window_set_title(window: *mut GtkWindow, title: *const c_char);
    fn gtk_window_set_transient_for(window: *mut GtkWindow, parent: *mut GtkWindow);

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

unsafe fn label_text_or_empty(text: *const c_char) -> *const c_char {
    if text.is_null() {
        b"\0".as_ptr() as *const c_char
    } else {
        text
    }
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

fn is_valid_channel_name_bytes(channel: &[u8]) -> bool {
    channel
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

unsafe fn page_name_for_page(page: c_int) -> *const c_char {
    if page == SETTINGS_WINDOW_PAGE_CHANNELS {
        b"channels\0".as_ptr() as *const c_char
    } else {
        b"general\0".as_ptr() as *const c_char
    }
}

unsafe extern "C" fn settings_window_free(data: *mut c_void) {
    let view = data as *mut SettingsWindow;
    if view.is_null() {
        return;
    }

    if !(*view).auth_cancel.is_null() {
        g_cancellable_cancel((*view).auth_cancel);
        g_object_unref((*view).auth_cancel as *mut c_void);
    }
    drop(Box::from_raw(view));
}

unsafe extern "C" fn channel_row_free(data: *mut c_void) {
    if !data.is_null() {
        drop(Box::from_raw(data as *mut ChannelRow));
    }
}

unsafe extern "C" fn g_free_destroy(data: *mut c_void) {
    g_free(data);
}

unsafe extern "C" fn g_object_unref_destroy(data: *mut c_void) {
    g_object_unref(data);
}

unsafe fn has_twitch_auth_client() -> bool {
    TWITCH_AUTH_CLIENT_ID[0] != 0
}

unsafe extern "C" fn on_settings_window_close_request(
    _window: *mut GtkWindow,
    user_data: *mut c_void,
) -> c_int {
    let view = user_data as *mut SettingsWindow;
    if !(*view).auth_cancel.is_null() {
        g_cancellable_cancel((*view).auth_cancel);
    }
    0
}

unsafe fn has_twitch_auth(view: *mut SettingsWindow) -> bool {
    let token = app_settings_get_twitch_oauth_token((*view).settings);
    let refresh_token = app_settings_get_twitch_refresh_token((*view).settings);
    is_nonempty(token) && is_nonempty(refresh_token)
}

unsafe fn set_twitch_auth_status(view: *mut SettingsWindow, message: *const c_char) {
    if !(*view).twitch_auth_status.is_null() {
        gtk_label_set_use_markup((*view).twitch_auth_status as *mut GtkLabel, 0);
        gtk_label_set_text(
            (*view).twitch_auth_status as *mut GtkLabel,
            label_text_or_empty(message),
        );
    }
}

unsafe fn set_twitch_auth_status_markup(view: *mut SettingsWindow, markup: *const c_char) {
    if !(*view).twitch_auth_status.is_null() {
        gtk_label_set_use_markup((*view).twitch_auth_status as *mut GtkLabel, 1);
        gtk_label_set_markup(
            (*view).twitch_auth_status as *mut GtkLabel,
            label_text_or_empty(markup),
        );
    }
}

unsafe extern "C" fn on_twitch_auth_status_link_activated(
    _label: *mut GtkLabel,
    uri: *const c_char,
    user_data: *mut c_void,
) -> c_int {
    let view = user_data as *mut SettingsWindow;
    if view.is_null() || (*view).window.is_null() || !is_nonempty(uri) {
        return 1;
    }

    let launcher = gtk_uri_launcher_new(uri);
    gtk_uri_launcher_launch(
        launcher,
        (*view).window as *mut GtkWindow,
        ptr::null_mut(),
        None,
        ptr::null_mut(),
    );
    g_object_unref(launcher as *mut c_void);
    1
}

unsafe fn update_twitch_auth_controls(view: *mut SettingsWindow) {
    if (*view).twitch_auth_button.is_null() {
        return;
    }

    let authenticated = has_twitch_auth(view);
    gtk_button_set_label(
        (*view).twitch_auth_button as *mut GtkButton,
        if authenticated {
            b"Disconnect Twitch\0".as_ptr() as *const c_char
        } else {
            b"Connect Twitch\0".as_ptr() as *const c_char
        },
    );
    gtk_widget_set_sensitive(
        (*view).twitch_auth_button,
        if has_twitch_auth_client() && (*view).auth_in_progress == 0 {
            1
        } else {
            0
        },
    );
}

unsafe fn update_empty_state(view: *mut SettingsWindow) {
    let has_rows = !gtk_widget_get_first_child((*view).channels_box).is_null();
    gtk_widget_set_visible((*view).empty_label, if has_rows { 0 } else { 1 });
}

unsafe extern "C" fn clear_channel_focus_after_remove(user_data: *mut c_void) -> c_int {
    let window = user_data as *mut GtkWidget;
    let view = g_object_get_data(
        window as *mut GObject,
        b"settings-window\0".as_ptr() as *const c_char,
    ) as *mut SettingsWindow;

    if view.is_null() {
        return G_SOURCE_REMOVE;
    }

    gtk_window_set_focus((*view).window as *mut GtkWindow, ptr::null_mut());
    let mut child = gtk_widget_get_first_child((*view).channels_box);
    while !child.is_null() {
        let row = g_object_get_data(
            child as *mut GObject,
            b"channel-row\0".as_ptr() as *const c_char,
        ) as *mut ChannelRow;
        if !row.is_null() {
            gtk_editable_select_region((*row).channel_entry as *mut GtkEditable, 0, 0);
        }
        child = gtk_widget_get_next_sibling(child);
    }

    G_SOURCE_REMOVE
}

unsafe extern "C" fn on_remove_channel_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let row = user_data as *mut ChannelRow;
    let view = (*row).view;
    let row_widget = (*row).row;

    gtk_box_remove((*view).channels_box as *mut GtkBox, row_widget);
    update_empty_state(view);
    g_idle_add_full(
        G_PRIORITY_DEFAULT_IDLE,
        Some(clear_channel_focus_after_remove),
        g_object_ref((*view).window as *mut c_void),
        Some(g_object_unref_destroy),
    );
}

unsafe fn create_channel_row(view: *mut SettingsWindow, channel: *const c_char) -> *mut GtkWidget {
    let row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    gtk_widget_add_css_class(row, b"settings-channel-row\0".as_ptr() as *const c_char);

    let row_data = Box::into_raw(Box::new(ChannelRow {
        view,
        row,
        channel_entry: ptr::null_mut(),
    }));
    g_object_set_data_full(
        row as *mut GObject,
        b"channel-row\0".as_ptr() as *const c_char,
        row_data as *mut c_void,
        Some(channel_row_free),
    );

    (*row_data).channel_entry = gtk_entry_new();
    gtk_entry_set_placeholder_text(
        (*row_data).channel_entry as *mut GtkEntry,
        b"Twitch Channel\0".as_ptr() as *const c_char,
    );
    gtk_editable_set_text(
        (*row_data).channel_entry as *mut GtkEditable,
        label_text_or_empty(channel),
    );
    gtk_widget_set_hexpand((*row_data).channel_entry, 1);
    gtk_box_append(row as *mut GtkBox, (*row_data).channel_entry);

    let remove_button = gtk_button_new();
    gtk_button_set_child(remove_button as *mut GtkButton, player_trash_icon_new());
    gtk_button_set_has_frame(remove_button as *mut GtkButton, 0);
    gtk_widget_add_css_class(
        remove_button,
        b"settings-remove-button\0".as_ptr() as *const c_char,
    );
    gtk_widget_set_tooltip_text(remove_button, b"Remove\0".as_ptr() as *const c_char);
    gtk_box_append(row as *mut GtkBox, remove_button);
    g_signal_connect_data(
        remove_button as *mut c_void,
        b"clicked\0".as_ptr() as *const c_char,
        on_remove_channel_clicked as *const c_void,
        row_data as *mut c_void,
        ptr::null_mut(),
        0,
    );

    row
}

unsafe fn add_channel_row(view: *mut SettingsWindow, channel: *const c_char) {
    gtk_box_append(
        (*view).channels_box as *mut GtkBox,
        create_channel_row(view, channel),
    );
    update_empty_state(view);
}

unsafe extern "C" fn on_add_channel_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    add_channel_row(
        user_data as *mut SettingsWindow,
        b"\0".as_ptr() as *const c_char,
    );
}

unsafe fn finish_twitch_auth(view: *mut SettingsWindow) {
    (*view).auth_in_progress = 0;
    if !(*view).auth_cancel.is_null() {
        g_object_unref((*view).auth_cancel as *mut c_void);
        (*view).auth_cancel = ptr::null_mut();
    }
    update_twitch_auth_controls(view);
}

unsafe fn notify_settings_saved(view: *mut SettingsWindow) {
    if let Some(callback) = (*view).saved_callback {
        callback((*view).settings, (*view).user_data);
    }
}

unsafe fn set_status_with_prefix(view: *mut SettingsWindow, prefix: &[u8], error: *mut GError) {
    let message = if error.is_null() || (*error).message.is_null() {
        b"unknown error".as_slice()
    } else {
        CStr::from_ptr((*error).message).to_bytes()
    };
    let mut text = Vec::with_capacity(prefix.len() + message.len() + 1);
    text.extend_from_slice(prefix);
    text.extend_from_slice(message);
    text.push(0);
    set_twitch_auth_status(view, text.as_ptr() as *const c_char);
}

unsafe extern "C" fn on_twitch_token_ready(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let window = user_data as *mut GtkWidget;
    let view = g_object_get_data(
        window as *mut GObject,
        b"settings-window\0".as_ptr() as *const c_char,
    ) as *mut SettingsWindow;
    let mut error: *mut GError = ptr::null_mut();
    let token = twitch_auth_poll_device_token_finish(result, &mut error);

    if view.is_null() {
        if !token.is_null() {
            twitch_auth_token_free(token);
        }
        g_clear_error(&mut error);
        g_object_unref(window as *mut c_void);
        return;
    }

    if token.is_null() {
        if !error.is_null() && g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0
        {
            set_status_with_prefix(view, b"Twitch login failed: ", error);
        }
        g_clear_error(&mut error);
        finish_twitch_auth(view);
        g_object_unref(window as *mut c_void);
        return;
    }

    let expires_at = if (*token).expires_in > 0 {
        g_get_real_time() / 1_000_000 + (*token).expires_in as i64
    } else {
        0
    };
    app_settings_set_twitch_auth_tokens(
        (*view).settings,
        (*token).access_token,
        (*token).refresh_token,
        expires_at,
    );

    if app_settings_save((*view).settings, &mut error) == 0 {
        set_status_with_prefix(
            view,
            b"Twitch login saved in memory, but saving failed: ",
            error,
        );
        g_clear_error(&mut error);
    } else {
        twitch_channel_list_invalidate_followed_cache();
        set_twitch_auth_status(
            view,
            b"Twitch connected. Followed channels are enabled.\0".as_ptr() as *const c_char,
        );
        notify_settings_saved(view);
    }

    twitch_auth_token_free(token);
    finish_twitch_auth(view);
    g_object_unref(window as *mut c_void);
}

unsafe extern "C" fn on_twitch_device_code_ready(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    let window = user_data as *mut GtkWidget;
    let view = g_object_get_data(
        window as *mut GObject,
        b"settings-window\0".as_ptr() as *const c_char,
    ) as *mut SettingsWindow;
    let mut error: *mut GError = ptr::null_mut();
    let code = twitch_auth_request_device_code_finish(result, &mut error);

    if view.is_null() {
        if !code.is_null() {
            twitch_auth_device_code_free(code);
        }
        g_clear_error(&mut error);
        g_object_unref(window as *mut c_void);
        return;
    }

    if code.is_null() {
        if !error.is_null() && g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0
        {
            set_status_with_prefix(view, b"Twitch login could not start: ", error);
        }
        g_clear_error(&mut error);
        finish_twitch_auth(view);
        g_object_unref(window as *mut c_void);
        return;
    }

    let launcher = gtk_uri_launcher_new((*code).verification_uri);
    gtk_uri_launcher_launch(
        launcher,
        (*view).window as *mut GtkWindow,
        ptr::null_mut(),
        None,
        ptr::null_mut(),
    );
    g_object_unref(launcher as *mut c_void);

    let escaped_uri = g_markup_escape_text((*code).verification_uri, -1);
    let escaped_code = g_markup_escape_text((*code).user_code, -1);
    let mut message = Vec::new();
    message.extend_from_slice(b"Open Twitch activation and enter code <a href=\"");
    message.extend_from_slice(CStr::from_ptr(escaped_uri).to_bytes());
    message.extend_from_slice(b"\">");
    message.extend_from_slice(CStr::from_ptr(escaped_code).to_bytes());
    message.extend_from_slice(b"</a>.\0");
    set_twitch_auth_status_markup(view, message.as_ptr() as *const c_char);
    g_free(escaped_uri as *mut c_void);
    g_free(escaped_code as *mut c_void);

    twitch_auth_poll_device_token_async(
        TWITCH_AUTH_CLIENT_ID.as_ptr() as *const c_char,
        code,
        (*view).auth_cancel,
        Some(on_twitch_token_ready),
        g_object_ref(window as *mut c_void),
    );
    twitch_auth_device_code_free(code);
    g_object_unref(window as *mut c_void);
}

unsafe fn disconnect_twitch(view: *mut SettingsWindow) {
    if !(*view).auth_cancel.is_null() {
        g_cancellable_cancel((*view).auth_cancel);
        g_object_unref((*view).auth_cancel as *mut c_void);
        (*view).auth_cancel = ptr::null_mut();
    }
    (*view).auth_in_progress = 0;

    app_settings_set_twitch_oauth_token((*view).settings, ptr::null());
    twitch_channel_list_invalidate_followed_cache();

    let mut error: *mut GError = ptr::null_mut();
    if app_settings_save((*view).settings, &mut error) == 0 {
        set_status_with_prefix(
            view,
            b"Twitch disconnected in memory, but saving failed: ",
            error,
        );
        g_clear_error(&mut error);
    } else {
        set_twitch_auth_status(view, b"Twitch disconnected.\0".as_ptr() as *const c_char);
        notify_settings_saved(view);
    }

    update_twitch_auth_controls(view);
}

unsafe extern "C" fn on_twitch_auth_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let view = user_data as *mut SettingsWindow;

    if has_twitch_auth(view) {
        disconnect_twitch(view);
        return;
    }

    if !has_twitch_auth_client() {
        set_twitch_auth_status(
            view,
            b"Twitch login is not configured for this build.\0".as_ptr() as *const c_char,
        );
        return;
    }

    if !(*view).auth_cancel.is_null() {
        g_cancellable_cancel((*view).auth_cancel);
        g_object_unref((*view).auth_cancel as *mut c_void);
    }

    (*view).auth_cancel = g_cancellable_new();
    (*view).auth_in_progress = 1;
    update_twitch_auth_controls(view);
    set_twitch_auth_status(
        view,
        b"Requesting Twitch login code...\0".as_ptr() as *const c_char,
    );

    twitch_auth_request_device_code_async(
        TWITCH_AUTH_CLIENT_ID.as_ptr() as *const c_char,
        (*view).auth_cancel,
        Some(on_twitch_device_code_ready),
        g_object_ref((*view).window as *mut c_void),
    );
}

unsafe extern "C" fn on_save_clicked(_button: *mut GtkButton, user_data: *mut c_void) {
    let view = user_data as *mut SettingsWindow;

    gtk_label_set_text(
        (*view).status_label as *mut GtkLabel,
        b"\0".as_ptr() as *const c_char,
    );
    let mut channels: Vec<Vec<u8>> = Vec::new();
    let mut child = gtk_widget_get_first_child((*view).channels_box);
    while !child.is_null() {
        let row = g_object_get_data(
            child as *mut GObject,
            b"channel-row\0".as_ptr() as *const c_char,
        ) as *mut ChannelRow;
        if !row.is_null() {
            let channel = gtk_editable_get_text((*row).channel_entry as *mut GtkEditable);
            let trimmed = trim_ascii(CStr::from_ptr(channel).to_bytes());
            if !trimmed.is_empty() {
                if !is_valid_channel_name_bytes(trimmed) {
                    gtk_label_set_text(
                        (*view).status_label as *mut GtkLabel,
                        b"Invalid channel name. Use a-z, 0-9 and _ only.\0".as_ptr()
                            as *const c_char,
                    );
                    return;
                }
                channels.push(trimmed.to_vec());
            }
        }
        child = gtk_widget_get_next_sibling(child);
    }

    app_settings_set_hwdec_enabled(
        (*view).settings,
        if (*view).hwdec_check.is_null()
            || gtk_check_button_get_active((*view).hwdec_check as *mut GtkCheckButton) != 0
        {
            1
        } else {
            0
        },
    );
    app_settings_clear_channels((*view).settings);
    for channel in channels {
        let channel = dup_bytes(&channel);
        app_settings_add_channel((*view).settings, ptr::null(), channel, ptr::null());
        g_free(channel as *mut c_void);
    }

    let mut error: *mut GError = ptr::null_mut();
    if app_settings_save((*view).settings, &mut error) == 0 {
        gtk_label_set_text((*view).status_label as *mut GtkLabel, (*error).message);
        g_clear_error(&mut error);
        return;
    }

    notify_settings_saved(view);
    gtk_window_close((*view).window as *mut GtkWindow);
}

unsafe fn create_sidebar_row(name: *const c_char, title: *const c_char) -> *mut GtkWidget {
    let row = gtk_list_box_row_new();
    let label = gtk_label_new(title);
    gtk_widget_add_css_class(label, b"settings-sidebar-label\0".as_ptr() as *const c_char);
    gtk_label_set_xalign(label as *mut GtkLabel, 0.0);
    gtk_list_box_row_set_child(row as *mut GtkListBoxRow, label);
    g_object_set_data_full(
        row as *mut GObject,
        b"settings-page\0".as_ptr() as *const c_char,
        g_strdup(name) as *mut c_void,
        Some(g_free_destroy),
    );
    row
}

unsafe extern "C" fn on_sidebar_row_selected(
    _box: *mut GtkListBox,
    row: *mut GtkListBoxRow,
    user_data: *mut c_void,
) {
    let view = user_data as *mut SettingsWindow;
    if row.is_null() || (*view).stack.is_null() {
        return;
    }

    let page = g_object_get_data(
        row as *mut GObject,
        b"settings-page\0".as_ptr() as *const c_char,
    ) as *const c_char;
    if !page.is_null() {
        gtk_stack_set_visible_child_name((*view).stack as *mut GtkStack, page);
    }
}

unsafe fn create_sidebar(view: *mut SettingsWindow, initial_page: c_int) -> *mut GtkWidget {
    let sidebar = gtk_list_box_new();
    gtk_widget_add_css_class(sidebar, b"settings-sidebar\0".as_ptr() as *const c_char);
    gtk_widget_set_size_request(sidebar, 170, -1);
    gtk_list_box_set_selection_mode(sidebar as *mut GtkListBox, GTK_SELECTION_SINGLE);

    let general_row = create_sidebar_row(
        b"general\0".as_ptr() as *const c_char,
        b"General\0".as_ptr() as *const c_char,
    );
    let channels_row = create_sidebar_row(
        b"channels\0".as_ptr() as *const c_char,
        b"Channels\0".as_ptr() as *const c_char,
    );
    gtk_list_box_append(sidebar as *mut GtkListBox, general_row);
    gtk_list_box_append(sidebar as *mut GtkListBox, channels_row);
    g_signal_connect_data(
        sidebar as *mut c_void,
        b"row-selected\0".as_ptr() as *const c_char,
        on_sidebar_row_selected as *const c_void,
        view as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_list_box_select_row(
        sidebar as *mut GtkListBox,
        if initial_page == SETTINGS_WINDOW_PAGE_CHANNELS {
            channels_row
        } else {
            general_row
        } as *mut GtkListBoxRow,
    );
    sidebar
}

unsafe fn create_general_page(view: *mut SettingsWindow) -> *mut GtkWidget {
    let page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 8);
    gtk_widget_add_css_class(page, b"settings-page\0".as_ptr() as *const c_char);
    gtk_widget_set_hexpand(page, 1);
    gtk_widget_set_vexpand(page, 1);

    let title = gtk_label_new(b"General\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(title, b"settings-page-title\0".as_ptr() as *const c_char);
    gtk_label_set_xalign(title as *mut GtkLabel, 0.0);
    gtk_box_append(page as *mut GtkBox, title);

    (*view).hwdec_check =
        gtk_check_button_new_with_label(b"Hardware decoding\0".as_ptr() as *const c_char);
    gtk_check_button_set_active(
        (*view).hwdec_check as *mut GtkCheckButton,
        app_settings_get_hwdec_enabled((*view).settings),
    );
    gtk_widget_add_css_class(
        (*view).hwdec_check,
        b"settings-check\0".as_ptr() as *const c_char,
    );
    gtk_box_append(page as *mut GtkBox, (*view).hwdec_check);

    let hint = gtk_label_new(b"Let mpv use GPU video decoding where supported. Disable this if playback is unstable or the video renders incorrectly.\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(hint, b"settings-hint-label\0".as_ptr() as *const c_char);
    gtk_label_set_xalign(hint as *mut GtkLabel, 0.0);
    gtk_label_set_wrap(hint as *mut GtkLabel, 1);
    gtk_widget_set_halign(hint, GTK_ALIGN_FILL);
    gtk_box_append(page as *mut GtkBox, hint);

    let spacer = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_set_vexpand(spacer, 1);
    gtk_box_append(page as *mut GtkBox, spacer);

    page
}

unsafe fn create_channels_page(view: *mut SettingsWindow) -> *mut GtkWidget {
    let page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 12);
    gtk_widget_add_css_class(page, b"settings-page\0".as_ptr() as *const c_char);
    gtk_widget_set_hexpand(page, 1);
    gtk_widget_set_vexpand(page, 1);

    let header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    let title = gtk_label_new(b"Channels\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(title, b"settings-page-title\0".as_ptr() as *const c_char);
    gtk_label_set_xalign(title as *mut GtkLabel, 0.0);
    gtk_widget_set_hexpand(title, 1);
    gtk_box_append(header as *mut GtkBox, title);
    gtk_box_append(page as *mut GtkBox, header);

    let twitch_section_title = gtk_label_new(b"Twitch account\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        twitch_section_title,
        b"settings-section-title\0".as_ptr() as *const c_char,
    );
    gtk_label_set_xalign(twitch_section_title as *mut GtkLabel, 0.0);
    gtk_box_append(page as *mut GtkBox, twitch_section_title);

    let followed_hint = gtk_label_new(b"Connect Twitch to include your followed channels in the channel switcher. Followings are cached for two minutes and are not saved into this list.\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        followed_hint,
        b"settings-hint-label\0".as_ptr() as *const c_char,
    );
    gtk_label_set_xalign(followed_hint as *mut GtkLabel, 0.0);
    gtk_label_set_wrap(followed_hint as *mut GtkLabel, 1);
    gtk_widget_set_halign(followed_hint, GTK_ALIGN_FILL);
    gtk_box_append(page as *mut GtkBox, followed_hint);

    let auth_box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    (*view).twitch_auth_button =
        gtk_button_new_with_label(b"Connect Twitch\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        (*view).twitch_auth_button,
        b"settings-primary-button\0".as_ptr() as *const c_char,
    );
    gtk_box_append(auth_box as *mut GtkBox, (*view).twitch_auth_button);
    g_signal_connect_data(
        (*view).twitch_auth_button as *mut c_void,
        b"clicked\0".as_ptr() as *const c_char,
        on_twitch_auth_clicked as *const c_void,
        view as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_box_append(page as *mut GtkBox, auth_box);

    (*view).twitch_auth_status = gtk_label_new(b"\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        (*view).twitch_auth_status,
        b"settings-hint-label\0".as_ptr() as *const c_char,
    );
    gtk_label_set_xalign((*view).twitch_auth_status as *mut GtkLabel, 0.0);
    gtk_label_set_wrap((*view).twitch_auth_status as *mut GtkLabel, 1);
    gtk_widget_set_halign((*view).twitch_auth_status, GTK_ALIGN_FILL);
    g_signal_connect_data(
        (*view).twitch_auth_status as *mut c_void,
        b"activate-link\0".as_ptr() as *const c_char,
        on_twitch_auth_status_link_activated as *const c_void,
        view as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_box_append(page as *mut GtkBox, (*view).twitch_auth_status);
    if !has_twitch_auth_client() {
        set_twitch_auth_status(
            view,
            b"Twitch login is not configured.\0".as_ptr() as *const c_char,
        );
    } else if has_twitch_auth(view) {
        set_twitch_auth_status(view, b"Twitch connected.\0".as_ptr() as *const c_char);
    } else if !app_settings_get_twitch_oauth_token((*view).settings).is_null() {
        set_twitch_auth_status(
            view,
            b"Reconnect Twitch to enable token refresh.\0".as_ptr() as *const c_char,
        );
    }
    update_twitch_auth_controls(view);

    let custom_header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(
        custom_header,
        b"settings-section-header\0".as_ptr() as *const c_char,
    );
    let custom_title = gtk_label_new(b"Custom channels\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        custom_title,
        b"settings-section-title\0".as_ptr() as *const c_char,
    );
    gtk_label_set_xalign(custom_title as *mut GtkLabel, 0.0);
    gtk_widget_set_hexpand(custom_title, 1);
    gtk_box_append(custom_header as *mut GtkBox, custom_title);

    let add_button = gtk_button_new_with_label(b"Add\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        add_button,
        b"settings-primary-button\0".as_ptr() as *const c_char,
    );
    gtk_box_append(custom_header as *mut GtkBox, add_button);
    g_signal_connect_data(
        add_button as *mut c_void,
        b"clicked\0".as_ptr() as *const c_char,
        on_add_channel_clicked as *const c_void,
        view as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_box_append(page as *mut GtkBox, custom_header);

    let custom_rule = gtk_separator_new(GTK_ORIENTATION_HORIZONTAL);
    gtk_widget_add_css_class(
        custom_rule,
        b"settings-section-rule\0".as_ptr() as *const c_char,
    );
    gtk_box_append(page as *mut GtkBox, custom_rule);

    (*view).empty_label = gtk_label_new(b"No channels saved yet.\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        (*view).empty_label,
        b"settings-empty-label\0".as_ptr() as *const c_char,
    );
    gtk_widget_set_halign((*view).empty_label, GTK_ALIGN_CENTER);
    gtk_label_set_xalign((*view).empty_label as *mut GtkLabel, 0.5);
    gtk_box_append(page as *mut GtkBox, (*view).empty_label);

    (*view).channels_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_widget_add_css_class(
        (*view).channels_box,
        b"settings-channels-box\0".as_ptr() as *const c_char,
    );
    gtk_widget_set_vexpand((*view).channels_box, 0);
    gtk_box_append(page as *mut GtkBox, (*view).channels_box);

    page
}

unsafe fn create_footer(view: *mut SettingsWindow) -> *mut GtkWidget {
    let footer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(footer, b"settings-footer\0".as_ptr() as *const c_char);
    (*view).status_label = gtk_label_new(b"\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        (*view).status_label,
        b"settings-status-label\0".as_ptr() as *const c_char,
    );
    gtk_label_set_xalign((*view).status_label as *mut GtkLabel, 0.0);
    gtk_widget_set_hexpand((*view).status_label, 1);
    gtk_box_append(footer as *mut GtkBox, (*view).status_label);

    let save_button = gtk_button_new_with_label(b"Save\0".as_ptr() as *const c_char);
    gtk_widget_add_css_class(
        save_button,
        b"settings-primary-button\0".as_ptr() as *const c_char,
    );
    gtk_box_append(footer as *mut GtkBox, save_button);
    g_signal_connect_data(
        save_button as *mut c_void,
        b"clicked\0".as_ptr() as *const c_char,
        on_save_clicked as *const c_void,
        view as *mut c_void,
        ptr::null_mut(),
        0,
    );

    footer
}

pub unsafe fn settings_window_show<W>(
    parent: *mut W,
    settings: *mut AppSettings,
    initial_page: c_int,
    saved_callback: Option<SettingsWindowSavedCallback>,
    user_data: *mut c_void,
) {
    let parent = parent as *mut GtkWindow;
    let view = Box::into_raw(Box::new(SettingsWindow {
        window: ptr::null_mut(),
        sidebar: ptr::null_mut(),
        stack: ptr::null_mut(),
        hwdec_check: ptr::null_mut(),
        twitch_auth_button: ptr::null_mut(),
        twitch_auth_status: ptr::null_mut(),
        channels_box: ptr::null_mut(),
        empty_label: ptr::null_mut(),
        status_label: ptr::null_mut(),
        settings,
        saved_callback,
        user_data,
        auth_cancel: ptr::null_mut(),
        auth_in_progress: 0,
    }));

    (*view).window = gtk_window_new();
    gtk_window_set_title(
        (*view).window as *mut GtkWindow,
        b"Settings\0".as_ptr() as *const c_char,
    );
    gtk_window_set_default_size((*view).window as *mut GtkWindow, 760, 480);
    gtk_window_set_modal((*view).window as *mut GtkWindow, 1);
    gtk_window_set_transient_for((*view).window as *mut GtkWindow, parent);
    g_object_set_data_full(
        (*view).window as *mut GObject,
        b"settings-window\0".as_ptr() as *const c_char,
        view as *mut c_void,
        Some(settings_window_free),
    );
    g_signal_connect_data(
        (*view).window as *mut c_void,
        b"close-request\0".as_ptr() as *const c_char,
        on_settings_window_close_request as *const c_void,
        view as *mut c_void,
        ptr::null_mut(),
        0,
    );

    let root = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_add_css_class(root, b"settings-window\0".as_ptr() as *const c_char);
    gtk_window_set_child((*view).window as *mut GtkWindow, root);

    (*view).stack = gtk_stack_new();
    gtk_widget_set_hexpand((*view).stack, 1);
    gtk_widget_set_vexpand((*view).stack, 1);
    gtk_stack_add_named(
        (*view).stack as *mut GtkStack,
        create_general_page(view),
        b"general\0".as_ptr() as *const c_char,
    );
    gtk_stack_add_named(
        (*view).stack as *mut GtkStack,
        create_channels_page(view),
        b"channels\0".as_ptr() as *const c_char,
    );

    let content = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_set_hexpand(content, 1);
    gtk_widget_set_vexpand(content, 1);
    let scrolled = gtk_scrolled_window_new();
    gtk_widget_set_hexpand(scrolled, 1);
    gtk_widget_set_vexpand(scrolled, 1);
    gtk_scrolled_window_set_policy(
        scrolled as *mut GtkScrolledWindow,
        GTK_POLICY_NEVER,
        GTK_POLICY_AUTOMATIC,
    );
    gtk_scrolled_window_set_child(scrolled as *mut GtkScrolledWindow, (*view).stack);
    gtk_box_append(content as *mut GtkBox, scrolled);
    gtk_box_append(content as *mut GtkBox, create_footer(view));

    (*view).sidebar = create_sidebar(view, initial_page);
    gtk_box_append(root as *mut GtkBox, (*view).sidebar);
    gtk_box_append(root as *mut GtkBox, content);

    for i in 0..app_settings_get_channel_count(settings) {
        let channel = app_settings_get_channel(settings, i);
        if !channel.is_null() {
            add_channel_row(view, (*channel).channel);
        }
    }
    update_empty_state(view);
    gtk_stack_set_visible_child_name(
        (*view).stack as *mut GtkStack,
        page_name_for_page(initial_page),
    );

    gtk_window_present((*view).window as *mut GtkWindow);
}
