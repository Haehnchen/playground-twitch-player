mod auth;
mod channels;
mod ffi;
mod pages;

pub use self::ffi::{
    GObject, GtkBox, GtkButton, GtkCheckButton, GtkEditable, GtkEntry, GtkLabel, GtkListBox,
    GtkListBoxRow, GtkScrolledWindow, GtkStack, GtkUriLauncher, GtkWidget, GtkWindow,
};

use self::channels::{add_channel_row, update_empty_state};
use self::ffi::{
    GTK_ORIENTATION_HORIZONTAL, GTK_ORIENTATION_VERTICAL, GTK_POLICY_AUTOMATIC, GTK_POLICY_NEVER,
    g_cancellable_cancel, g_free, g_object_set_data_full, g_object_unref, g_signal_connect_data,
    gtk_box_append, gtk_box_new, gtk_scrolled_window_new, gtk_scrolled_window_set_child,
    gtk_scrolled_window_set_policy, gtk_stack_add_named, gtk_stack_new,
    gtk_stack_set_visible_child_name, gtk_widget_add_css_class, gtk_widget_set_hexpand,
    gtk_widget_set_vexpand, gtk_window_new, gtk_window_present, gtk_window_set_child,
    gtk_window_set_default_size, gtk_window_set_modal, gtk_window_set_title,
    gtk_window_set_transient_for,
};
use self::pages::{
    create_about_page, create_channels_page, create_footer, create_general_page, create_sidebar,
    page_name_for_page,
};
use crate::settings::{
    AppSettings, app_settings_free, app_settings_get_channel, app_settings_get_channel_count,
    app_settings_ref,
};
use crate::twitch::stream_info::{GAsyncResult, GCancellable};
use std::ffi::{c_int, c_void};
use std::ptr;

pub struct SettingsWindow {
    window: *mut GtkWidget,
    sidebar: *mut GtkWidget,
    stack: *mut GtkWidget,
    hwdec_check: *mut GtkWidget,
    twitch_auth_button: *mut GtkWidget,
    twitch_auth_status: *mut GtkWidget,
    twitch_playback_auth_entry: *mut GtkWidget,
    channels_box: *mut GtkWidget,
    empty_label: *mut GtkWidget,
    status_label: *mut GtkWidget,

    settings: *mut AppSettings,

    saved_callback: Option<SettingsWindowSavedCallback>,
    user_data: *mut c_void,
    user_data_free: Option<GDestroyNotify>,

    auth_cancel: *mut GCancellable,
    auth_in_progress: c_int,
}

pub struct ChannelRow {
    view: *mut SettingsWindow,
    row: *mut GtkWidget,
    channel_entry: *mut GtkWidget,
}

type GAsyncReadyCallback = unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void);

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

pub type SettingsWindowSavedCallback = unsafe extern "C" fn(*mut AppSettings, *mut c_void);

unsafe extern "C" fn settings_window_free(data: *mut c_void) {
    unsafe {
        let view = data as *mut SettingsWindow;
        if view.is_null() {
            return;
        }

        if !(*view).auth_cancel.is_null() {
            g_cancellable_cancel((*view).auth_cancel);
            g_object_unref((*view).auth_cancel as *mut c_void);
        }
        app_settings_free((*view).settings);
        if let Some(user_data_free) = (*view).user_data_free {
            user_data_free((*view).user_data);
        }
        drop(Box::from_raw(view));
    }
}

unsafe extern "C" fn g_free_destroy(data: *mut c_void) {
    unsafe {
        g_free(data);
    }
}

unsafe extern "C" fn g_object_unref_destroy(data: *mut c_void) {
    unsafe {
        g_object_unref(data);
    }
}

unsafe extern "C" fn on_settings_window_close_request(
    _window: *mut GtkWindow,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        let view = user_data as *mut SettingsWindow;
        if !(*view).auth_cancel.is_null() {
            g_cancellable_cancel((*view).auth_cancel);
        }
        0
    }
}

unsafe fn notify_settings_saved(view: *mut SettingsWindow) {
    unsafe {
        if let Some(callback) = (*view).saved_callback {
            callback((*view).settings, (*view).user_data);
        }
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `parent`: null or live `GtkWindow`; `settings`: live, serialized access.
/// - Callbacks: declared C ABI, no unwind; data valid until dialog destruction.
/// - Supplying `user_data_free` transfers callback-data ownership to the dialog.
pub unsafe fn settings_window_show<W>(
    parent: *mut W,
    settings: *mut AppSettings,
    initial_page: c_int,
    saved_callback: Option<SettingsWindowSavedCallback>,
    user_data: *mut c_void,
    user_data_free: Option<GDestroyNotify>,
) {
    unsafe {
        let parent = parent as *mut GtkWindow;
        let view = Box::into_raw(Box::new(SettingsWindow {
            window: ptr::null_mut(),
            sidebar: ptr::null_mut(),
            stack: ptr::null_mut(),
            hwdec_check: ptr::null_mut(),
            twitch_auth_button: ptr::null_mut(),
            twitch_auth_status: ptr::null_mut(),
            twitch_playback_auth_entry: ptr::null_mut(),
            channels_box: ptr::null_mut(),
            empty_label: ptr::null_mut(),
            status_label: ptr::null_mut(),

            settings: app_settings_ref(settings),

            saved_callback,
            user_data,
            user_data_free,

            auth_cancel: ptr::null_mut(),
            auth_in_progress: 0,
        }));

        (*view).window = gtk_window_new();
        gtk_window_set_title((*view).window as *mut GtkWindow, c"Settings".as_ptr());
        gtk_window_set_default_size((*view).window as *mut GtkWindow, 760, 480);
        gtk_window_set_modal((*view).window as *mut GtkWindow, 1);
        gtk_window_set_transient_for((*view).window as *mut GtkWindow, parent);
        g_object_set_data_full(
            (*view).window as *mut GObject,
            c"settings-window".as_ptr(),
            view as *mut c_void,
            Some(settings_window_free),
        );
        g_signal_connect_data(
            (*view).window as *mut c_void,
            c"close-request".as_ptr(),
            on_settings_window_close_request as *const c_void,
            view as *mut c_void,
            ptr::null_mut(),
            0,
        );

        let root = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
        gtk_widget_add_css_class(root, c"settings-window".as_ptr());
        gtk_window_set_child((*view).window as *mut GtkWindow, root);

        (*view).stack = gtk_stack_new();
        gtk_widget_set_hexpand((*view).stack, 1);
        gtk_widget_set_vexpand((*view).stack, 1);
        gtk_stack_add_named(
            (*view).stack as *mut GtkStack,
            create_general_page(view),
            c"general".as_ptr(),
        );
        gtk_stack_add_named(
            (*view).stack as *mut GtkStack,
            create_channels_page(view),
            c"channels".as_ptr(),
        );
        gtk_stack_add_named(
            (*view).stack as *mut GtkStack,
            create_about_page(),
            c"about".as_ptr(),
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
}
