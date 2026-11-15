mod auth;
mod channels;
mod ffi;
mod pages;

pub use self::ffi::{
    GObject, GtkBox, GtkButton, GtkCheckButton, GtkEditable, GtkEntry, GtkLabel, GtkListBox,
    GtkListBoxRow, GtkScrolledWindow, GtkStack, GtkUriLauncher, GtkWidget, GtkWindow,
};

use self::channels::{add_channel_row, update_empty_state};
use self::ffi::{g_cancellable_cancel, g_object_set_data_full, g_object_unref};
use self::pages::{
    create_about_page, create_channels_page, create_footer, create_general_page, create_sidebar,
    page_name_for_page,
};
use crate::settings::{
    AppSettings, app_settings_free, app_settings_get_channel, app_settings_get_channel_count,
    app_settings_ref,
};
use crate::twitch::stream_info::{GAsyncResult, GCancellable};
use gtk4::{
    glib::{self, translate::*},
    prelude::*,
};
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

type GAsyncReadyCallback = unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void);

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

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

        let window = gtk4::Window::builder()
            .title("Settings")
            .default_width(760)
            .default_height(480)
            .modal(true)
            .build();
        if !parent.is_null() {
            let parent: glib::translate::Borrowed<gtk4::Window> = from_glib_borrow(parent.cast());
            window.set_transient_for(Some(&*parent));
        }
        (*view).window = window.as_ptr().cast();
        g_object_set_data_full(
            window.as_ptr().cast(),
            c"settings-window".as_ptr(),
            view.cast(),
            Some(settings_window_free),
        );
        window.connect_close_request(move |_| {
            if !(*view).auth_cancel.is_null() {
                g_cancellable_cancel((*view).auth_cancel);
            }
            glib::Propagation::Proceed
        });
        let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        root.add_css_class("settings-window");
        window.set_child(Some(&root));
        let stack = gtk4::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        (*view).stack = stack.as_ptr().cast();
        stack.add_named(&create_general_page(view), Some("general"));
        stack.add_named(&create_channels_page(view), Some("channels"));
        stack.add_named(&create_about_page(), Some("about"));
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_vexpand(true);
        let scrolled = gtk4::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();
        scrolled.set_child(Some(&stack));
        content.append(&scrolled);
        content.append(&create_footer(view));
        let sidebar = create_sidebar(view, initial_page);
        (*view).sidebar = sidebar.as_ptr().cast();
        root.append(&sidebar);
        root.append(&content);
        for i in 0..app_settings_get_channel_count(settings) {
            let channel = app_settings_get_channel(settings, i);
            if !channel.is_null() {
                add_channel_row(view, (*channel).channel);
            }
        }
        update_empty_state(view);
        stack.set_visible_child_name(page_name_for_page(initial_page));
        window.present();
    }
}
