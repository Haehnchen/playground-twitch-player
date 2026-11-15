use super::ffi::{
    G_IO_ERROR_CANCELLED, GObject, GtkButton, GtkLabel, GtkWidget, GtkWindow, g_cancellable_cancel,
    g_cancellable_new, g_clear_error, g_error_matches, g_free, g_get_real_time, g_io_error_quark,
    g_markup_escape_text, g_object_get_data, g_object_ref, g_object_unref, gtk_button_set_label,
    gtk_label_set_markup, gtk_label_set_text, gtk_label_set_use_markup, gtk_uri_launcher_launch,
    gtk_uri_launcher_new, gtk_widget_set_sensitive,
};
use super::pages::label_text_or_empty;
use super::{SettingsWindow, notify_settings_saved};
use crate::glib_strings::is_nonempty;
use crate::settings::{
    app_settings_get_twitch_oauth_token, app_settings_get_twitch_refresh_token, app_settings_save,
    app_settings_set_twitch_auth_tokens, app_settings_set_twitch_oauth_token,
};
use crate::twitch::auth::{
    twitch_auth_device_code_free, twitch_auth_poll_device_token_async,
    twitch_auth_poll_device_token_finish, twitch_auth_request_device_code_async,
    twitch_auth_request_device_code_finish, twitch_auth_token_free,
};
use crate::twitch::channel_list::twitch_channel_list_invalidate_followed_cache;
use crate::twitch::stream_info::{GAsyncResult, GError};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
pub(super) const TWITCH_AUTH_CLIENT_ID: &[u8] = b"8l1fzyh4jhs1cxhtqs6p4swmxuejh6\0";

pub(super) unsafe fn has_twitch_auth_client() -> bool {
    TWITCH_AUTH_CLIENT_ID[0] != 0
}

pub(super) unsafe fn has_twitch_auth(view: *mut SettingsWindow) -> bool {
    unsafe {
        let token = app_settings_get_twitch_oauth_token((*view).settings);
        let refresh_token = app_settings_get_twitch_refresh_token((*view).settings);
        is_nonempty(token) && is_nonempty(refresh_token)
    }
}

pub(super) unsafe fn set_twitch_auth_status(view: *mut SettingsWindow, message: *const c_char) {
    unsafe {
        if !(*view).twitch_auth_status.is_null() {
            gtk_label_set_use_markup((*view).twitch_auth_status as *mut GtkLabel, 0);
            gtk_label_set_text(
                (*view).twitch_auth_status as *mut GtkLabel,
                label_text_or_empty(message),
            );
        }
    }
}

pub(super) unsafe fn set_twitch_auth_status_markup(
    view: *mut SettingsWindow,
    markup: *const c_char,
) {
    unsafe {
        if !(*view).twitch_auth_status.is_null() {
            gtk_label_set_use_markup((*view).twitch_auth_status as *mut GtkLabel, 1);
            gtk_label_set_markup(
                (*view).twitch_auth_status as *mut GtkLabel,
                label_text_or_empty(markup),
            );
        }
    }
}

pub(super) unsafe extern "C" fn on_twitch_auth_status_link_activated(
    _label: *mut GtkLabel,
    uri: *const c_char,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
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
}

pub(super) unsafe fn update_twitch_auth_controls(view: *mut SettingsWindow) {
    unsafe {
        if (*view).twitch_auth_button.is_null() {
            return;
        }

        let authenticated = has_twitch_auth(view);
        gtk_button_set_label(
            (*view).twitch_auth_button as *mut GtkButton,
            if authenticated {
                c"Disconnect Twitch".as_ptr()
            } else {
                c"Connect Twitch".as_ptr()
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
}

pub(super) unsafe fn finish_twitch_auth(view: *mut SettingsWindow) {
    unsafe {
        (*view).auth_in_progress = 0;
        if !(*view).auth_cancel.is_null() {
            g_object_unref((*view).auth_cancel as *mut c_void);
            (*view).auth_cancel = ptr::null_mut();
        }
        update_twitch_auth_controls(view);
    }
}

pub(super) unsafe fn set_status_with_prefix(
    view: *mut SettingsWindow,
    prefix: &[u8],
    error: *mut GError,
) {
    unsafe {
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
}

// Save completed Twitch authorization and update the dialog.
pub(super) unsafe extern "C" fn on_twitch_token_ready(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let window = user_data as *mut GtkWidget;
        let view = g_object_get_data(window as *mut GObject, c"settings-window".as_ptr())
            as *mut SettingsWindow;
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
            if !error.is_null()
                && g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0
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
                c"Twitch connected. Followed channels are enabled.".as_ptr(),
            );
            notify_settings_saved(view);
        }

        twitch_auth_token_free(token);
        finish_twitch_auth(view);
        g_object_unref(window as *mut c_void);
    }
}

pub(super) unsafe extern "C" fn on_twitch_device_code_ready(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let window = user_data as *mut GtkWidget;
        let view = g_object_get_data(window as *mut GObject, c"settings-window".as_ptr())
            as *mut SettingsWindow;
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
            if !error.is_null()
                && g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0
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
            TWITCH_AUTH_CLIENT_ID.as_ptr().cast(),
            code,
            (*view).auth_cancel,
            Some(on_twitch_token_ready),
            g_object_ref(window as *mut c_void),
        );
        twitch_auth_device_code_free(code);
        g_object_unref(window as *mut c_void);
    }
}

pub(super) unsafe fn disconnect_twitch(view: *mut SettingsWindow) {
    unsafe {
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
            set_twitch_auth_status(view, c"Twitch disconnected.".as_ptr());
            notify_settings_saved(view);
        }

        update_twitch_auth_controls(view);
    }
}

pub(super) unsafe extern "C" fn on_twitch_auth_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let view = user_data as *mut SettingsWindow;

        if has_twitch_auth(view) {
            disconnect_twitch(view);
            return;
        }

        if !has_twitch_auth_client() {
            set_twitch_auth_status(
                view,
                c"Twitch login is not configured for this build.".as_ptr(),
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
        set_twitch_auth_status(view, c"Requesting Twitch login code...".as_ptr());

        twitch_auth_request_device_code_async(
            TWITCH_AUTH_CLIENT_ID.as_ptr().cast(),
            (*view).auth_cancel,
            Some(on_twitch_device_code_ready),
            g_object_ref((*view).window as *mut c_void),
        );
    }
}
