use super::ChannelSwitcherOverlay;
use super::actions::{
    current_search_filter, find_settings_channel, on_backdrop_pressed, on_channel_button_clicked,
    on_close_clicked, on_direct_channel_activate, on_direct_channel_icon_pressed,
    on_search_activate, on_search_changed, on_settings_clicked, preview_matches_filter,
};
use super::ffi::*;
use super::images::load_remote_image;
use super::layout::{AVATAR_SIZE, CARD_SPACING, current_card_layout, get_grid_columns};
use super::runtime::{
    add_switcher_timeout, add_weak_pointer, connect_switcher_signal, g_free_destroy, preview_at,
    remove_source_if_active, switcher_is_closing,
};
use crate::glib_strings::is_nonempty;
use crate::player::icons::{player_settings_icon_new, player_window_icon_new};
use crate::twitch::stream_info::{
    TwitchStreamPreview, twitch_stream_info_format_live_duration,
    twitch_stream_info_format_viewer_count,
};
use gtk4::{glib::translate::from_glib_none, prelude::*};
use std::ffi::{c_char, c_int, c_uint, c_void};

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
        gtk_widget_add_controller((*switcher).backdrop, backdrop_click.cast());
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

const PREVIEW_CARD_BUILD_BATCH_SIZE: c_uint = 2;

const PREVIEW_CARD_BUILD_INTERVAL_MS: c_uint = 8;

// Use a timer so video redraws cannot starve card creation.
pub(super) unsafe fn queue_preview_card_batch(switcher: *mut ChannelSwitcherOverlay) -> c_uint {
    unsafe {
        add_switcher_timeout(
            PREVIEW_CARD_BUILD_INTERVAL_MS,
            Some(build_preview_card_batch),
            switcher,
        )
    }
}

pub(super) unsafe fn clear_grid(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
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
}

pub(super) unsafe fn clear_preview_cards(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        remove_source_if_active(&mut (*switcher).preview_card_build_source);
        clear_grid(switcher);
        (*switcher).preview_cards.clear();
        (*switcher).preview_card_columns = 0;
        (*switcher).preview_card_next_index = 0;
        (*switcher).preview_card_visible_count = 0;
        (*switcher).preview_card_width = 0;
        (*switcher).preview_width = 0;
        (*switcher).preview_height = 0;
    }
}

pub(super) unsafe fn show_status(switcher: *mut ChannelSwitcherOverlay, message: *const c_char) {
    unsafe {
        super::radial::show_status(switcher, message);
        if (*switcher).grid.is_null() {
            return;
        }

        clear_grid(switcher);

        let label = gtk_label_new(message);
        gtk_widget_add_css_class(label, c"channel-switcher-status".as_ptr());
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
}

unsafe fn create_image_picture(
    switcher: *mut ChannelSwitcherOverlay,
    url: *const c_char,
    width: c_int,
    height: c_int,
    css_class: *const c_char,
) -> *mut GtkWidget {
    unsafe {
        let image = gtk_picture_new();

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
        load_remote_image(switcher, image, url, width, height);
        image
    }
}

unsafe fn create_fixed_picture_frame(
    picture: *mut GtkWidget,
    width: c_int,
    height: c_int,
) -> *mut GtkWidget {
    unsafe {
        let frame = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);

        gtk_widget_add_css_class(frame, c"channel-switcher-image-frame".as_ptr());
        gtk_widget_set_size_request(frame, width, height);
        gtk_widget_set_halign(frame, GTK_ALIGN_START);
        gtk_widget_set_valign(frame, GTK_ALIGN_START);
        gtk_widget_set_hexpand(frame, FALSE);
        gtk_widget_set_vexpand(frame, FALSE);
        gtk_widget_set_overflow(frame, GTK_OVERFLOW_HIDDEN);
        gtk_box_append(frame as *mut GtkBox, picture);

        frame
    }
}

unsafe fn format_viewer_count(viewer_count: c_uint) -> *mut c_char {
    unsafe {
        let count = twitch_stream_info_format_viewer_count(viewer_count);
        let result = g_strdup_printf(c"%s viewers".as_ptr(), count);
        g_free(count as *mut c_void);
        result
    }
}

unsafe fn format_meta_text(preview: *const TwitchStreamPreview) -> *mut c_char {
    unsafe {
        let viewers = format_viewer_count((*preview).viewer_count);
        let duration = twitch_stream_info_format_live_duration((*preview).started_at.as_ptr());
        let result = g_strdup_printf(
            c"%s \u{2022} %s".as_ptr(),
            viewers,
            if duration.is_null() {
                c"live".as_ptr()
            } else {
                duration
            },
        );
        g_free(viewers as *mut c_void);
        g_free(duration as *mut c_void);
        result
    }
}

unsafe fn create_channel_card(
    switcher: *mut ChannelSwitcherOverlay,
    preview: *const TwitchStreamPreview,
    card_width: c_int,
    preview_width: c_int,
    preview_height: c_int,
) -> *mut GtkWidget {
    unsafe {
        let channel = find_settings_channel(switcher, (*preview).channel.as_ptr());
        let label = if !channel.is_null() && is_nonempty((*channel).label.as_ptr()) {
            (*channel).label.as_ptr()
        } else {
            (*preview).display_name.as_ptr()
        };

        let button = gtk_button_new();
        gtk_widget_add_css_class(button, c"channel-switcher-item".as_ptr());
        gtk_widget_set_halign(button, GTK_ALIGN_START);
        gtk_widget_set_hexpand(button, FALSE);
        gtk_widget_set_size_request(button, card_width, -1);
        g_object_set_data_full(
            button as *mut GObject,
            c"channel-name".as_ptr(),
            g_strdup((*preview).channel.as_ptr()) as *mut c_void,
            Some(g_free_destroy),
        );
        g_object_set_data_full(
            button as *mut GObject,
            c"channel-label".as_ptr(),
            g_strdup(label) as *mut c_void,
            Some(g_free_destroy),
        );
        connect_switcher_signal(
            button as *mut c_void,
            c"clicked".as_ptr(),
            on_channel_button_clicked as *const c_void,
            switcher,
        );

        let card = gtk_box_new(GTK_ORIENTATION_VERTICAL, 5);
        gtk_widget_set_halign(card, GTK_ALIGN_START);
        gtk_widget_set_hexpand(card, FALSE);
        gtk_widget_set_size_request(card, card_width, -1);

        let preview_frame = create_fixed_picture_frame(
            create_image_picture(
                switcher,
                (*preview).preview_url.as_ptr(),
                preview_width,
                preview_height,
                c"channel-switcher-preview".as_ptr(),
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
                (*preview).avatar_url.as_ptr(),
                AVATAR_SIZE,
                AVATAR_SIZE,
                c"channel-switcher-avatar".as_ptr(),
            ),
            AVATAR_SIZE,
            AVATAR_SIZE,
        );

        let title_label = gtk_label_new(if (*preview).title.is_empty() {
            c"".as_ptr()
        } else {
            (*preview).title.as_ptr()
        });
        gtk_widget_add_css_class(title_label, c"channel-switcher-title".as_ptr());
        gtk_label_set_xalign(title_label as *mut GtkLabel, 0.0);
        gtk_label_set_single_line_mode(title_label as *mut GtkLabel, TRUE);
        gtk_label_set_ellipsize(title_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
        gtk_label_set_max_width_chars(title_label as *mut GtkLabel, 26);
        gtk_widget_set_halign(title_label, GTK_ALIGN_FILL);

        let name_label = gtk_label_new(label);
        gtk_widget_add_css_class(name_label, c"channel-switcher-name".as_ptr());
        gtk_label_set_xalign(name_label as *mut GtkLabel, 0.0);
        gtk_label_set_ellipsize(name_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
        gtk_label_set_max_width_chars(name_label as *mut GtkLabel, 26);
        gtk_widget_set_halign(name_label, GTK_ALIGN_FILL);

        let meta_text = format_meta_text(preview);
        let meta_label = gtk_label_new(meta_text);
        gtk_widget_add_css_class(meta_label, c"channel-switcher-meta".as_ptr());
        gtk_label_set_xalign(meta_label as *mut GtkLabel, 0.0);
        gtk_label_set_ellipsize(meta_label as *mut GtkLabel, PANGO_ELLIPSIZE_END);
        gtk_label_set_max_width_chars(meta_label as *mut GtkLabel, 26);
        gtk_widget_set_halign(meta_label, GTK_ALIGN_FILL);
        g_free(meta_text as *mut c_void);

        gtk_box_append(text_box as *mut GtkBox, title_label);
        gtk_box_append(text_box as *mut GtkBox, name_label);
        gtk_box_append(text_box as *mut GtkBox, meta_label);
        if is_nonempty((*preview).category_name.as_ptr()) {
            let category_label = gtk_label_new((*preview).category_name.as_ptr());
            gtk_widget_add_css_class(category_label, c"channel-switcher-meta".as_ptr());
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
}

// Build and attach a small batch, then let GTK draw.
unsafe extern "C" fn build_preview_card_batch(user_data: *mut c_void) -> c_int {
    unsafe {
        let switcher = user_data as *mut ChannelSwitcherOverlay;
        if switcher_is_closing(switcher)
            || (*switcher).panel.is_null()
            || (*switcher).previews.is_none()
        {
            (*switcher).preview_card_build_source = 0;
            return G_SOURCE_REMOVE;
        }

        if gtk_widget_get_visible((*switcher).panel) == 0 {
            (*switcher).preview_card_build_source = 0;
            return G_SOURCE_REMOVE;
        }

        let start = (*switcher).preview_card_next_index;
        let end = ((*switcher).previews.as_ref().unwrap().len() as c_uint)
            .min(start.saturating_add(PREVIEW_CARD_BUILD_BATCH_SIZE));
        if start == 0 {
            clear_grid(switcher);
        }
        let filter = current_search_filter(switcher);
        for i in start..end {
            let preview = preview_at(switcher, i);
            if i as usize >= (*switcher).preview_cards.len() {
                let card = create_channel_card(
                    switcher,
                    preview,
                    (*switcher).preview_card_width,
                    (*switcher).preview_width,
                    (*switcher).preview_height,
                );
                (*switcher).preview_cards.push(from_glib_none(card.cast()));
            }
            if preview_matches_filter(preview, filter) {
                let visible_count = (*switcher).preview_card_visible_count;
                let columns = (*switcher).preview_card_columns;
                gtk_grid_attach(
                    (*switcher).grid as *mut GtkGrid,
                    (&(*switcher).preview_cards)[i as usize].as_ptr().cast(),
                    (visible_count % columns) as c_int,
                    (visible_count / columns) as c_int,
                    1,
                    1,
                );
                (*switcher).preview_card_visible_count += 1;
            }
        }
        (*switcher).preview_card_next_index = end;

        if end < (*switcher).previews.as_ref().unwrap().len() as c_uint {
            return G_SOURCE_CONTINUE;
        }

        (*switcher).preview_card_build_source = 0;
        if (*switcher).preview_card_visible_count == 0 {
            show_status(switcher, c"No live channels match the filter".as_ptr());
        }
        G_SOURCE_REMOVE
    }
}

unsafe fn prepare_preview_card_layout(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        let layout = current_card_layout(switcher);

        let layout_matches = (*switcher).preview_card_columns == layout.columns
            && (*switcher).preview_card_width == layout.card_width
            && (*switcher).preview_width == layout.preview_width
            && (*switcher).preview_height == layout.preview_height;
        if !layout_matches {
            clear_preview_cards(switcher);
            (*switcher).preview_card_columns = layout.columns;
            (*switcher).preview_card_width = layout.card_width;
            (*switcher).preview_width = layout.preview_width;
            (*switcher).preview_height = layout.preview_height;
        }
    }
}

pub(super) unsafe fn render_live_channels(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if (*switcher).panel.is_null() || gtk_widget_get_visible((*switcher).panel) == 0 {
            super::radial::render(switcher);
            return;
        }
        remove_source_if_active(&mut (*switcher).preview_card_build_source);
        if (*switcher).previews.is_none()
            || (*switcher).previews.as_ref().unwrap().len() as c_uint == 0
        {
            show_status(switcher, c"No configured channels are live".as_ptr());
            return;
        }

        prepare_preview_card_layout(switcher);
        (*switcher).preview_card_next_index = 0;
        (*switcher).preview_card_visible_count = 0;
        show_status(switcher, c"Preparing live channels".as_ptr());
        (*switcher).preview_card_build_source = queue_preview_card_batch(switcher);
    }
}
