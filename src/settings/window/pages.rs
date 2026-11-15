use super::SettingsWindow;
use super::auth::{
    has_twitch_auth, has_twitch_auth_client, on_twitch_auth_clicked,
    on_twitch_auth_status_link_activated, set_twitch_auth_status, update_twitch_auth_controls,
};
use super::channels::{on_add_channel_clicked, on_save_clicked};
use crate::settings::{
    app_settings_get_hwdec_enabled, app_settings_get_twitch_oauth_token,
    app_settings_get_twitch_playback_auth_token,
};
use gtk4::{glib, prelude::*};
use std::ffi::{CStr, CString, c_char, c_int};

pub(super) const SETTINGS_WINDOW_PAGE_CHANNELS: c_int = 1;
pub(super) const SETTINGS_WINDOW_PAGE_ABOUT: c_int = 2;

pub(super) fn label_text_or_empty(text: *const c_char) -> *const c_char {
    if text.is_null() { c"".as_ptr() } else { text }
}

pub(super) fn page_name_for_page(page: c_int) -> &'static str {
    match page {
        SETTINGS_WINDOW_PAGE_CHANNELS => "channels",
        SETTINGS_WINDOW_PAGE_ABOUT => "about",
        _ => "general",
    }
}

fn label(text: &str, class: &str) -> gtk4::Label {
    let label = gtk4::Label::new(Some(text));
    label.add_css_class(class);
    label.set_xalign(0.0);
    label
}

fn page(title: &str, spacing: i32) -> gtk4::Box {
    let page = gtk4::Box::new(gtk4::Orientation::Vertical, spacing);
    page.add_css_class("settings-page");
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.append(&label(title, "settings-page-title"));
    page
}

fn hint(text: &str) -> gtk4::Label {
    let hint = label(text, "settings-hint-label");
    hint.set_wrap(true);
    hint.set_halign(gtk4::Align::Fill);
    hint
}

unsafe fn connect_link(label: &gtk4::Label, view: *mut SettingsWindow) {
    label.connect_activate_link(move |label, uri| unsafe {
        let uri = CString::new(uri).unwrap();
        on_twitch_auth_status_link_activated(label.as_ptr().cast(), uri.as_ptr(), view.cast());
        glib::Propagation::Stop
    });
}

pub(super) unsafe fn create_sidebar(
    view: *mut SettingsWindow,
    initial_page: c_int,
) -> gtk4::ListBox {
    unsafe {
        let sidebar = gtk4::ListBox::new();
        sidebar.add_css_class("settings-sidebar");
        sidebar.set_size_request(170, -1);
        sidebar.set_selection_mode(gtk4::SelectionMode::Single);
        for (name, title) in [
            ("general", "General"),
            ("channels", "Channels"),
            ("about", "About"),
        ] {
            let row = gtk4::ListBoxRow::new();
            row.set_widget_name(name);
            row.set_child(Some(&label(title, "settings-sidebar-label")));
            sidebar.append(&row);
            if name == page_name_for_page(initial_page) {
                sidebar.select_row(Some(&row));
            }
        }
        let stack: gtk4::Stack = glib::translate::from_glib_none((*view).stack.cast());
        sidebar.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                stack.set_visible_child_name(&row.widget_name());
            }
        });
        sidebar
    }
}

pub(super) unsafe fn create_general_page(view: *mut SettingsWindow) -> gtk4::Box {
    unsafe {
        let page = page("General", 8);
        let hwdec = gtk4::CheckButton::with_label("Hardware decoding");
        hwdec.set_active(app_settings_get_hwdec_enabled((*view).settings) != 0);
        hwdec.add_css_class("settings-check");
        (*view).hwdec_check = hwdec.as_ptr().cast();
        page.append(&hwdec);
        page.append(&hint("Let mpv use GPU video decoding where supported. Disable this if playback is unstable or the video renders incorrectly."));
        let spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        page.append(&spacer);
        page
    }
}

pub(super) unsafe fn create_channels_page(view: *mut SettingsWindow) -> gtk4::Box {
    unsafe {
        let page = page("Channels", 12);
        page.append(&label("Twitch account", "settings-section-title"));
        page.append(&hint("Connect Twitch to include your followed channels in the channel switcher. Followings are cached for two minutes and are not saved into this list."));
        let auth_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let auth = gtk4::Button::with_label("Connect Twitch");
        auth.add_css_class("settings-primary-button");
        (*view).twitch_auth_button = auth.as_ptr().cast();
        auth.connect_clicked(move |button| {
            on_twitch_auth_clicked(button.as_ptr().cast(), view.cast())
        });
        auth_box.append(&auth);
        page.append(&auth_box);
        let status = hint("");
        (*view).twitch_auth_status = status.as_ptr().cast();
        connect_link(&status, view);
        page.append(&status);
        if !has_twitch_auth_client() {
            set_twitch_auth_status(view, c"Twitch login is not configured.".as_ptr());
        } else if has_twitch_auth(view) {
            set_twitch_auth_status(view, c"Twitch connected.".as_ptr());
        } else if !app_settings_get_twitch_oauth_token((*view).settings).is_null() {
            set_twitch_auth_status(view, c"Reconnect Twitch to enable token refresh.".as_ptr());
        }
        update_twitch_auth_controls(view);
        page.append(&label("Playback authentication", "settings-section-title"));
        let token = gtk4::Entry::new();
        token.add_css_class("settings-token-entry");
        token.set_placeholder_text(Some("Twitch auth-token"));
        token.set_visibility(false);
        token.set_text(
            &CStr::from_ptr(label_text_or_empty(
                app_settings_get_twitch_playback_auth_token((*view).settings),
            ))
            .to_string_lossy(),
        );
        token.set_hexpand(true);
        (*view).twitch_playback_auth_entry = token.as_ptr().cast();
        page.append(&token);
        let help = hint("");
        help.set_markup("Paste Twitch's website auth-token to use subscription or Turbo benefits during playback. <a href=\"https://streamlink.github.io/cli/plugins/twitch.html#authentication\">How to find the token</a>.");
        connect_link(&help, view);
        page.append(&help);
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header.add_css_class("settings-section-header");
        let title = label("Custom channels", "settings-section-title");
        title.set_hexpand(true);
        header.append(&title);
        let add = gtk4::Button::with_label("Add");
        add.add_css_class("settings-primary-button");
        add.connect_clicked(move |button| {
            on_add_channel_clicked(button.as_ptr().cast(), view.cast())
        });
        header.append(&add);
        page.append(&header);
        let rule = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        rule.add_css_class("settings-section-rule");
        page.append(&rule);
        let empty = label("No channels saved yet.", "settings-empty-label");
        empty.set_halign(gtk4::Align::Center);
        empty.set_xalign(0.5);
        (*view).empty_label = empty.as_ptr().cast();
        page.append(&empty);
        let channels = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        channels.add_css_class("settings-channels-box");
        (*view).channels_box = channels.as_ptr().cast();
        page.append(&channels);
        page
    }
}

pub(super) fn create_about_page() -> gtk4::Box {
    let page = page("About", 12);
    page.append(&label("Build", "settings-section-title"));
    for (title, value) in [
        ("Version", env!("TWITCH_PLAYER_BUILD_VERSION")),
        ("Build date", env!("TWITCH_PLAYER_BUILD_DATE")),
        ("Author", env!("TWITCH_PLAYER_AUTHOR")),
    ] {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        row.add_css_class("settings-about-row");
        let title = label(title, "settings-about-key");
        title.set_size_request(96, -1);
        row.append(&title);
        let value = label(value, "settings-about-value");
        value.set_wrap(true);
        value.set_hexpand(true);
        row.append(&value);
        page.append(&row);
    }
    let spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    page.append(&spacer);
    page
}

pub(super) unsafe fn create_footer(view: *mut SettingsWindow) -> gtk4::Box {
    unsafe {
        let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        footer.add_css_class("settings-footer");
        let status = label("", "settings-status-label");
        status.set_hexpand(true);
        (*view).status_label = status.as_ptr().cast();
        footer.append(&status);
        let save = gtk4::Button::with_label("Save");
        save.add_css_class("settings-primary-button");
        save.connect_clicked(move |button| on_save_clicked(button.as_ptr().cast(), view.cast()));
        footer.append(&save);
        footer
    }
}
