//! Picker widgets and signal wiring.

use super::ChannelSwitcherOverlay;
use super::actions::{
    on_backdrop_pressed, on_close_clicked, on_direct_channel_activate,
    on_direct_channel_icon_pressed, on_search_activate, on_search_changed, on_settings_clicked,
};
use super::ffi::*;
use super::layout::CARD_SPACING;
use super::runtime::{add_weak_pointer, connect_switcher_signal};
use crate::player_icons::{player_settings_icon_new, player_window_icon_new};
use std::ffi::{c_uint, c_void};

pub(super) unsafe fn build_widgets(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        let overlay = (*switcher).overlay;
        add_weak_pointer((*switcher).overlay, &mut (*switcher).overlay);

        (*switcher).backdrop = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        add_weak_pointer((*switcher).backdrop, &mut (*switcher).backdrop);
        gtk_widget_add_css_class((*switcher).backdrop, c"channel-switcher-backdrop".as_ptr());
        gtk_widget_set_halign((*switcher).backdrop, GTK_ALIGN_FILL);
        gtk_widget_set_valign((*switcher).backdrop, GTK_ALIGN_FILL);
        gtk_widget_set_hexpand((*switcher).backdrop, TRUE);
        gtk_widget_set_vexpand((*switcher).backdrop, TRUE);
        gtk_widget_set_visible((*switcher).backdrop, FALSE);
        let backdrop_click = gtk_gesture_click_new();
        gtk_gesture_single_set_button(backdrop_click as *mut GtkGestureSingle, GDK_BUTTON_PRIMARY);
        connect_switcher_signal(
            backdrop_click as *mut c_void,
            c"pressed".as_ptr(),
            on_backdrop_pressed as *const c_void,
            switcher,
        );
        gtk_widget_add_controller((*switcher).backdrop, backdrop_click as *mut c_void);
        gtk_overlay_add_overlay(overlay, (*switcher).backdrop);

        (*switcher).panel = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        add_weak_pointer((*switcher).panel, &mut (*switcher).panel);
        gtk_widget_add_css_class((*switcher).panel, c"channel-switcher-panel".as_ptr());
        gtk_widget_set_halign((*switcher).panel, GTK_ALIGN_CENTER);
        gtk_widget_set_valign((*switcher).panel, GTK_ALIGN_START);
        gtk_widget_set_hexpand((*switcher).panel, FALSE);
        gtk_widget_set_visible((*switcher).panel, FALSE);

        let header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
        gtk_widget_add_css_class(header, c"channel-switcher-header".as_ptr());
        (*switcher).search_entry = gtk_search_entry_new();
        add_weak_pointer((*switcher).search_entry, &mut (*switcher).search_entry);
        gtk_widget_add_css_class(
            (*switcher).search_entry,
            c"channel-switcher-search".as_ptr(),
        );
        gtk_search_entry_set_placeholder_text(
            (*switcher).search_entry as *mut GtkSearchEntry,
            c"Filter live channels".as_ptr(),
        );
        connect_switcher_signal(
            (*switcher).search_entry as *mut c_void,
            c"changed".as_ptr(),
            on_search_changed as *const c_void,
            switcher,
        );
        connect_switcher_signal(
            (*switcher).search_entry as *mut c_void,
            c"activate".as_ptr(),
            on_search_activate as *const c_void,
            switcher,
        );
        (*switcher).direct_channel_entry = gtk_entry_new();
        add_weak_pointer(
            (*switcher).direct_channel_entry,
            &mut (*switcher).direct_channel_entry,
        );
        gtk_widget_add_css_class(
            (*switcher).direct_channel_entry,
            c"channel-switcher-open-entry".as_ptr(),
        );
        gtk_entry_set_placeholder_text(
            (*switcher).direct_channel_entry as *mut GtkEntry,
            c"Channel or Twitch URL".as_ptr(),
        );
        gtk_entry_set_icon_from_icon_name(
            (*switcher).direct_channel_entry as *mut GtkEntry,
            GTK_ENTRY_ICON_SECONDARY,
            c"media-playback-start-symbolic".as_ptr(),
        );
        gtk_entry_set_icon_tooltip_text(
            (*switcher).direct_channel_entry as *mut GtkEntry,
            GTK_ENTRY_ICON_SECONDARY,
            c"Open channel".as_ptr(),
        );
        gtk_widget_set_tooltip_text(
            (*switcher).direct_channel_entry,
            c"Enter a channel name or Twitch URL".as_ptr(),
        );
        connect_switcher_signal(
            (*switcher).direct_channel_entry as *mut c_void,
            c"activate".as_ptr(),
            on_direct_channel_activate as *const c_void,
            switcher,
        );
        connect_switcher_signal(
            (*switcher).direct_channel_entry as *mut c_void,
            c"icon-press".as_ptr(),
            on_direct_channel_icon_pressed as *const c_void,
            switcher,
        );
        let header_spacer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
        gtk_widget_set_hexpand(header_spacer, TRUE);
        let settings_button = gtk_button_new();
        gtk_button_set_child(
            settings_button as *mut GtkButton,
            player_settings_icon_new(),
        );
        gtk_widget_add_css_class(settings_button, c"channel-switcher-action".as_ptr());
        gtk_widget_set_tooltip_text(settings_button, c"Edit channels".as_ptr());
        connect_switcher_signal(
            settings_button as *mut c_void,
            c"clicked".as_ptr(),
            on_settings_clicked as *const c_void,
            switcher,
        );
        let close_button = gtk_button_new();
        gtk_button_set_child(
            close_button as *mut GtkButton,
            player_window_icon_new(PLAYER_WINDOW_ICON_CLOSE),
        );
        gtk_widget_add_css_class(close_button, c"channel-switcher-close".as_ptr());
        gtk_widget_set_tooltip_text(close_button, c"Close".as_ptr());
        connect_switcher_signal(
            close_button as *mut c_void,
            c"clicked".as_ptr(),
            on_close_clicked as *const c_void,
            switcher,
        );
        let input_separator = gtk_label_new(c"|".as_ptr());
        gtk_widget_add_css_class(
            input_separator,
            c"channel-switcher-header-separator".as_ptr(),
        );
        gtk_box_append(header as *mut GtkBox, (*switcher).search_entry);
        gtk_box_append(header as *mut GtkBox, input_separator);
        gtk_box_append(header as *mut GtkBox, (*switcher).direct_channel_entry);
        gtk_box_append(header as *mut GtkBox, header_spacer);
        gtk_box_append(header as *mut GtkBox, settings_button);
        gtk_box_append(header as *mut GtkBox, close_button);
        gtk_box_append((*switcher).panel as *mut GtkBox, header);

        (*switcher).scroller = gtk_scrolled_window_new();
        add_weak_pointer((*switcher).scroller, &mut (*switcher).scroller);
        gtk_widget_add_css_class((*switcher).scroller, c"channel-switcher-scroller".as_ptr());
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
        gtk_widget_add_css_class((*switcher).grid, c"channel-switcher-grid".as_ptr());
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
    }
}
