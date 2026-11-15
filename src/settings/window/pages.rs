use super::auth::{
    has_twitch_auth, has_twitch_auth_client, on_twitch_auth_clicked,
    on_twitch_auth_status_link_activated, set_twitch_auth_status, update_twitch_auth_controls,
};
use super::channels::{on_add_channel_clicked, on_save_clicked};
use super::ffi::{
    GObject, GTK_ALIGN_CENTER, GTK_ALIGN_FILL, GTK_ORIENTATION_HORIZONTAL,
    GTK_ORIENTATION_VERTICAL, GTK_SELECTION_SINGLE, GtkBox, GtkCheckButton, GtkEditable, GtkEntry,
    GtkLabel, GtkListBox, GtkListBoxRow, GtkStack, GtkWidget, g_object_get_data,
    g_object_set_data_full, g_signal_connect_data, g_strdup, gtk_box_append, gtk_box_new,
    gtk_button_new_with_label, gtk_check_button_new_with_label, gtk_check_button_set_active,
    gtk_editable_set_text, gtk_entry_new, gtk_entry_set_placeholder_text, gtk_entry_set_visibility,
    gtk_label_new, gtk_label_set_markup, gtk_label_set_wrap, gtk_label_set_xalign,
    gtk_list_box_append, gtk_list_box_new, gtk_list_box_row_new, gtk_list_box_row_set_child,
    gtk_list_box_select_row, gtk_list_box_set_selection_mode, gtk_separator_new,
    gtk_stack_set_visible_child_name, gtk_widget_add_css_class, gtk_widget_set_halign,
    gtk_widget_set_hexpand, gtk_widget_set_size_request, gtk_widget_set_vexpand,
};
use super::{SettingsWindow, g_free_destroy};
use crate::settings::{
    app_settings_get_hwdec_enabled, app_settings_get_twitch_oauth_token,
    app_settings_get_twitch_playback_auth_token,
};
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
pub(super) const SETTINGS_WINDOW_PAGE_CHANNELS: c_int = 1;
pub(super) const SETTINGS_WINDOW_PAGE_ABOUT: c_int = 2;

pub(super) unsafe fn label_text_or_empty(text: *const c_char) -> *const c_char {
    if text.is_null() { c"".as_ptr() } else { text }
}

pub(super) unsafe fn page_name_for_page(page: c_int) -> *const c_char {
    if page == SETTINGS_WINDOW_PAGE_CHANNELS {
        c"channels".as_ptr()
    } else if page == SETTINGS_WINDOW_PAGE_ABOUT {
        c"about".as_ptr()
    } else {
        c"general".as_ptr()
    }
}

pub(super) unsafe fn create_sidebar_row(
    name: *const c_char,
    title: *const c_char,
) -> *mut GtkWidget {
    unsafe {
        let row = gtk_list_box_row_new();
        let label = gtk_label_new(title);
        gtk_widget_add_css_class(label, c"settings-sidebar-label".as_ptr());
        gtk_label_set_xalign(label as *mut GtkLabel, 0.0);
        gtk_list_box_row_set_child(row as *mut GtkListBoxRow, label);
        g_object_set_data_full(
            row as *mut GObject,
            c"settings-page".as_ptr(),
            g_strdup(name) as *mut c_void,
            Some(g_free_destroy),
        );
        row
    }
}

pub(super) unsafe extern "C" fn on_sidebar_row_selected(
    _box: *mut GtkListBox,
    row: *mut GtkListBoxRow,
    user_data: *mut c_void,
) {
    unsafe {
        let view = user_data as *mut SettingsWindow;
        if row.is_null() || (*view).stack.is_null() {
            return;
        }

        let page =
            g_object_get_data(row as *mut GObject, c"settings-page".as_ptr()) as *const c_char;
        if !page.is_null() {
            gtk_stack_set_visible_child_name((*view).stack as *mut GtkStack, page);
        }
    }
}

pub(super) unsafe fn create_sidebar(
    view: *mut SettingsWindow,
    initial_page: c_int,
) -> *mut GtkWidget {
    unsafe {
        let sidebar = gtk_list_box_new();
        gtk_widget_add_css_class(sidebar, c"settings-sidebar".as_ptr());
        gtk_widget_set_size_request(sidebar, 170, -1);
        gtk_list_box_set_selection_mode(sidebar as *mut GtkListBox, GTK_SELECTION_SINGLE);

        let general_row = create_sidebar_row(c"general".as_ptr(), c"General".as_ptr());
        let channels_row = create_sidebar_row(c"channels".as_ptr(), c"Channels".as_ptr());
        let about_row = create_sidebar_row(c"about".as_ptr(), c"About".as_ptr());
        gtk_list_box_append(sidebar as *mut GtkListBox, general_row);
        gtk_list_box_append(sidebar as *mut GtkListBox, channels_row);
        gtk_list_box_append(sidebar as *mut GtkListBox, about_row);
        g_signal_connect_data(
            sidebar as *mut c_void,
            c"row-selected".as_ptr(),
            on_sidebar_row_selected as *const c_void,
            view as *mut c_void,
            ptr::null_mut(),
            0,
        );
        gtk_list_box_select_row(
            sidebar as *mut GtkListBox,
            if initial_page == SETTINGS_WINDOW_PAGE_ABOUT {
                about_row
            } else if initial_page == SETTINGS_WINDOW_PAGE_CHANNELS {
                channels_row
            } else {
                general_row
            } as *mut GtkListBoxRow,
        );
        sidebar
    }
}

pub(super) unsafe fn create_general_page(view: *mut SettingsWindow) -> *mut GtkWidget {
    unsafe {
        let page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 8);
        gtk_widget_add_css_class(page, c"settings-page".as_ptr());
        gtk_widget_set_hexpand(page, 1);
        gtk_widget_set_vexpand(page, 1);

        let title = gtk_label_new(c"General".as_ptr());
        gtk_widget_add_css_class(title, c"settings-page-title".as_ptr());
        gtk_label_set_xalign(title as *mut GtkLabel, 0.0);
        gtk_box_append(page as *mut GtkBox, title);

        (*view).hwdec_check = gtk_check_button_new_with_label(c"Hardware decoding".as_ptr());
        gtk_check_button_set_active(
            (*view).hwdec_check as *mut GtkCheckButton,
            app_settings_get_hwdec_enabled((*view).settings),
        );
        gtk_widget_add_css_class((*view).hwdec_check, c"settings-check".as_ptr());
        gtk_box_append(page as *mut GtkBox, (*view).hwdec_check);

        let hint = gtk_label_new(c"Let mpv use GPU video decoding where supported. Disable this if playback is unstable or the video renders incorrectly.".as_ptr());
        gtk_widget_add_css_class(hint, c"settings-hint-label".as_ptr());
        gtk_label_set_xalign(hint as *mut GtkLabel, 0.0);
        gtk_label_set_wrap(hint as *mut GtkLabel, 1);
        gtk_widget_set_halign(hint, GTK_ALIGN_FILL);
        gtk_box_append(page as *mut GtkBox, hint);

        let spacer = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        gtk_widget_set_vexpand(spacer, 1);
        gtk_box_append(page as *mut GtkBox, spacer);

        page
    }
}

pub(super) unsafe fn create_channels_page(view: *mut SettingsWindow) -> *mut GtkWidget {
    unsafe {
        let page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 12);
        gtk_widget_add_css_class(page, c"settings-page".as_ptr());
        gtk_widget_set_hexpand(page, 1);
        gtk_widget_set_vexpand(page, 1);

        let header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
        let title = gtk_label_new(c"Channels".as_ptr());
        gtk_widget_add_css_class(title, c"settings-page-title".as_ptr());
        gtk_label_set_xalign(title as *mut GtkLabel, 0.0);
        gtk_widget_set_hexpand(title, 1);
        gtk_box_append(header as *mut GtkBox, title);
        gtk_box_append(page as *mut GtkBox, header);

        let twitch_section_title = gtk_label_new(c"Twitch account".as_ptr());
        gtk_widget_add_css_class(twitch_section_title, c"settings-section-title".as_ptr());
        gtk_label_set_xalign(twitch_section_title as *mut GtkLabel, 0.0);
        gtk_box_append(page as *mut GtkBox, twitch_section_title);

        let followed_hint = gtk_label_new(c"Connect Twitch to include your followed channels in the channel switcher. Followings are cached for two minutes and are not saved into this list.".as_ptr());
        gtk_widget_add_css_class(followed_hint, c"settings-hint-label".as_ptr());
        gtk_label_set_xalign(followed_hint as *mut GtkLabel, 0.0);
        gtk_label_set_wrap(followed_hint as *mut GtkLabel, 1);
        gtk_widget_set_halign(followed_hint, GTK_ALIGN_FILL);
        gtk_box_append(page as *mut GtkBox, followed_hint);

        let auth_box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
        (*view).twitch_auth_button = gtk_button_new_with_label(c"Connect Twitch".as_ptr());
        gtk_widget_add_css_class(
            (*view).twitch_auth_button,
            c"settings-primary-button".as_ptr(),
        );
        gtk_box_append(auth_box as *mut GtkBox, (*view).twitch_auth_button);
        g_signal_connect_data(
            (*view).twitch_auth_button as *mut c_void,
            c"clicked".as_ptr(),
            on_twitch_auth_clicked as *const c_void,
            view as *mut c_void,
            ptr::null_mut(),
            0,
        );
        gtk_box_append(page as *mut GtkBox, auth_box);

        (*view).twitch_auth_status = gtk_label_new(c"".as_ptr());
        gtk_widget_add_css_class((*view).twitch_auth_status, c"settings-hint-label".as_ptr());
        gtk_label_set_xalign((*view).twitch_auth_status as *mut GtkLabel, 0.0);
        gtk_label_set_wrap((*view).twitch_auth_status as *mut GtkLabel, 1);
        gtk_widget_set_halign((*view).twitch_auth_status, GTK_ALIGN_FILL);
        g_signal_connect_data(
            (*view).twitch_auth_status as *mut c_void,
            c"activate-link".as_ptr(),
            on_twitch_auth_status_link_activated as *const c_void,
            view as *mut c_void,
            ptr::null_mut(),
            0,
        );
        gtk_box_append(page as *mut GtkBox, (*view).twitch_auth_status);
        if !has_twitch_auth_client() {
            set_twitch_auth_status(view, c"Twitch login is not configured.".as_ptr());
        } else if has_twitch_auth(view) {
            set_twitch_auth_status(view, c"Twitch connected.".as_ptr());
        } else if !app_settings_get_twitch_oauth_token((*view).settings).is_null() {
            set_twitch_auth_status(view, c"Reconnect Twitch to enable token refresh.".as_ptr());
        }
        update_twitch_auth_controls(view);

        let playback_auth_title = gtk_label_new(c"Playback authentication".as_ptr());
        gtk_widget_add_css_class(playback_auth_title, c"settings-section-title".as_ptr());
        gtk_label_set_xalign(playback_auth_title as *mut GtkLabel, 0.0);
        gtk_box_append(page as *mut GtkBox, playback_auth_title);

        (*view).twitch_playback_auth_entry = gtk_entry_new();
        gtk_widget_add_css_class(
            (*view).twitch_playback_auth_entry,
            c"settings-token-entry".as_ptr(),
        );
        gtk_entry_set_placeholder_text(
            (*view).twitch_playback_auth_entry as *mut GtkEntry,
            c"Twitch auth-token".as_ptr(),
        );
        gtk_entry_set_visibility((*view).twitch_playback_auth_entry as *mut GtkEntry, 0);
        gtk_editable_set_text(
            (*view).twitch_playback_auth_entry as *mut GtkEditable,
            label_text_or_empty(app_settings_get_twitch_playback_auth_token(
                (*view).settings,
            )),
        );
        gtk_widget_set_hexpand((*view).twitch_playback_auth_entry, 1);
        gtk_box_append(page as *mut GtkBox, (*view).twitch_playback_auth_entry);

        let playback_auth_hint = gtk_label_new(ptr::null());
        gtk_widget_add_css_class(playback_auth_hint, c"settings-hint-label".as_ptr());
        gtk_label_set_markup(
        playback_auth_hint as *mut GtkLabel,
        c"Paste Twitch's website auth-token to use subscription or Turbo benefits during playback. <a href=\"https://streamlink.github.io/cli/plugins/twitch.html#authentication\">How to find the token</a>."
            .as_ptr(),
    );
        gtk_label_set_xalign(playback_auth_hint as *mut GtkLabel, 0.0);
        gtk_label_set_wrap(playback_auth_hint as *mut GtkLabel, 1);
        gtk_widget_set_halign(playback_auth_hint, GTK_ALIGN_FILL);
        g_signal_connect_data(
            playback_auth_hint as *mut c_void,
            c"activate-link".as_ptr(),
            on_twitch_auth_status_link_activated as *const c_void,
            view as *mut c_void,
            ptr::null_mut(),
            0,
        );
        gtk_box_append(page as *mut GtkBox, playback_auth_hint);

        let custom_header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
        gtk_widget_add_css_class(custom_header, c"settings-section-header".as_ptr());
        let custom_title = gtk_label_new(c"Custom channels".as_ptr());
        gtk_widget_add_css_class(custom_title, c"settings-section-title".as_ptr());
        gtk_label_set_xalign(custom_title as *mut GtkLabel, 0.0);
        gtk_widget_set_hexpand(custom_title, 1);
        gtk_box_append(custom_header as *mut GtkBox, custom_title);

        let add_button = gtk_button_new_with_label(c"Add".as_ptr());
        gtk_widget_add_css_class(add_button, c"settings-primary-button".as_ptr());
        gtk_box_append(custom_header as *mut GtkBox, add_button);
        g_signal_connect_data(
            add_button as *mut c_void,
            c"clicked".as_ptr(),
            on_add_channel_clicked as *const c_void,
            view as *mut c_void,
            ptr::null_mut(),
            0,
        );
        gtk_box_append(page as *mut GtkBox, custom_header);

        let custom_rule = gtk_separator_new(GTK_ORIENTATION_HORIZONTAL);
        gtk_widget_add_css_class(custom_rule, c"settings-section-rule".as_ptr());
        gtk_box_append(page as *mut GtkBox, custom_rule);

        (*view).empty_label = gtk_label_new(c"No channels saved yet.".as_ptr());
        gtk_widget_add_css_class((*view).empty_label, c"settings-empty-label".as_ptr());
        gtk_widget_set_halign((*view).empty_label, GTK_ALIGN_CENTER);
        gtk_label_set_xalign((*view).empty_label as *mut GtkLabel, 0.5);
        gtk_box_append(page as *mut GtkBox, (*view).empty_label);

        (*view).channels_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
        gtk_widget_add_css_class((*view).channels_box, c"settings-channels-box".as_ptr());
        gtk_widget_set_vexpand((*view).channels_box, 0);
        gtk_box_append(page as *mut GtkBox, (*view).channels_box);

        page
    }
}

pub(super) unsafe fn create_about_row(
    title: *const c_char,
    value: *const c_char,
) -> *mut GtkWidget {
    unsafe {
        let row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 12);
        gtk_widget_add_css_class(row, c"settings-about-row".as_ptr());

        let title_label = gtk_label_new(title);
        gtk_widget_add_css_class(title_label, c"settings-about-key".as_ptr());
        gtk_label_set_xalign(title_label as *mut GtkLabel, 0.0);
        gtk_widget_set_size_request(title_label, 96, -1);
        gtk_box_append(row as *mut GtkBox, title_label);

        let value_label = gtk_label_new(value);
        gtk_widget_add_css_class(value_label, c"settings-about-value".as_ptr());
        gtk_label_set_xalign(value_label as *mut GtkLabel, 0.0);
        gtk_label_set_wrap(value_label as *mut GtkLabel, 1);
        gtk_widget_set_hexpand(value_label, 1);
        gtk_box_append(row as *mut GtkBox, value_label);

        row
    }
}

pub(super) unsafe fn create_about_page() -> *mut GtkWidget {
    unsafe {
        let page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 12);
        gtk_widget_add_css_class(page, c"settings-page".as_ptr());
        gtk_widget_set_hexpand(page, 1);
        gtk_widget_set_vexpand(page, 1);

        let title = gtk_label_new(c"About".as_ptr());
        gtk_widget_add_css_class(title, c"settings-page-title".as_ptr());
        gtk_label_set_xalign(title as *mut GtkLabel, 0.0);
        gtk_box_append(page as *mut GtkBox, title);

        let build_title = gtk_label_new(c"Build".as_ptr());
        gtk_widget_add_css_class(build_title, c"settings-section-title".as_ptr());
        gtk_label_set_xalign(build_title as *mut GtkLabel, 0.0);
        gtk_box_append(page as *mut GtkBox, build_title);

        gtk_box_append(
            page as *mut GtkBox,
            create_about_row(
                c"Version".as_ptr(),
                concat!(env!("TWITCH_PLAYER_BUILD_VERSION"), "\0")
                    .as_ptr()
                    .cast(),
            ),
        );
        gtk_box_append(
            page as *mut GtkBox,
            create_about_row(
                c"Build date".as_ptr(),
                concat!(env!("TWITCH_PLAYER_BUILD_DATE"), "\0")
                    .as_ptr()
                    .cast(),
            ),
        );
        gtk_box_append(
            page as *mut GtkBox,
            create_about_row(
                c"Author".as_ptr(),
                concat!(env!("TWITCH_PLAYER_AUTHOR"), "\0").as_ptr().cast(),
            ),
        );

        let spacer = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        gtk_widget_set_vexpand(spacer, 1);
        gtk_box_append(page as *mut GtkBox, spacer);

        page
    }
}

pub(super) unsafe fn create_footer(view: *mut SettingsWindow) -> *mut GtkWidget {
    unsafe {
        let footer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
        gtk_widget_add_css_class(footer, c"settings-footer".as_ptr());
        (*view).status_label = gtk_label_new(c"".as_ptr());
        gtk_widget_add_css_class((*view).status_label, c"settings-status-label".as_ptr());
        gtk_label_set_xalign((*view).status_label as *mut GtkLabel, 0.0);
        gtk_widget_set_hexpand((*view).status_label, 1);
        gtk_box_append(footer as *mut GtkBox, (*view).status_label);

        let save_button = gtk_button_new_with_label(c"Save".as_ptr());
        gtk_widget_add_css_class(save_button, c"settings-primary-button".as_ptr());
        gtk_box_append(footer as *mut GtkBox, save_button);
        g_signal_connect_data(
            save_button as *mut c_void,
            c"clicked".as_ptr(),
            on_save_clicked as *const c_void,
            view as *mut c_void,
            ptr::null_mut(),
            0,
        );

        footer
    }
}
