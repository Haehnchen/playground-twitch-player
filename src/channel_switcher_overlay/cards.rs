//! Preview cards and batch rendering.

use super::ChannelSwitcherOverlay;
use super::actions::{
    current_search_filter, find_settings_channel, on_channel_button_clicked, preview_matches_filter,
};
use super::ffi::*;
use super::images::load_remote_image;
use super::layout::{AVATAR_SIZE, current_card_layout, get_grid_columns};
use super::runtime::{
    add_switcher_timeout, connect_switcher_signal, g_free_destroy, g_object_unref_destroy,
    ptr_array_index, remove_source_if_active, switcher_is_closing,
};
use crate::glib_strings::is_nonempty;
use crate::twitch_stream_info::{
    TwitchStreamPreview, twitch_stream_info_format_live_duration,
    twitch_stream_info_format_viewer_count,
};
use std::ffi::{c_char, c_int, c_uint, c_void};

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
        if !(*switcher).preview_cards.is_null() {
            g_ptr_array_set_size((*switcher).preview_cards, 0);
        }
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

unsafe fn format_meta_text(preview: *mut TwitchStreamPreview) -> *mut c_char {
    unsafe {
        let viewers = format_viewer_count((*preview).viewer_count);
        let duration = twitch_stream_info_format_live_duration((*preview).started_at);
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
    preview: *mut TwitchStreamPreview,
    card_width: c_int,
    preview_width: c_int,
    preview_height: c_int,
) -> *mut GtkWidget {
    unsafe {
        let channel = find_settings_channel(switcher, (*preview).channel);
        let label = if !channel.is_null() && is_nonempty((*channel).label) {
            (*channel).label
        } else {
            (*preview).display_name
        };

        let button = gtk_button_new();
        gtk_widget_add_css_class(button, c"channel-switcher-item".as_ptr());
        gtk_widget_set_halign(button, GTK_ALIGN_START);
        gtk_widget_set_hexpand(button, FALSE);
        gtk_widget_set_size_request(button, card_width, -1);
        g_object_set_data_full(
            button as *mut GObject,
            c"channel-name".as_ptr(),
            g_strdup((*preview).channel) as *mut c_void,
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
                (*preview).preview_url,
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
                (*preview).avatar_url,
                AVATAR_SIZE,
                AVATAR_SIZE,
                c"channel-switcher-avatar".as_ptr(),
            ),
            AVATAR_SIZE,
            AVATAR_SIZE,
        );

        let title_label = gtk_label_new(if (*preview).title.is_null() {
            c"".as_ptr()
        } else {
            (*preview).title
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
        if is_nonempty((*preview).category_name) {
            let category_label = gtk_label_new((*preview).category_name);
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
            || (*switcher).previews.is_null()
            || (*switcher).preview_cards.is_null()
        {
            (*switcher).preview_card_build_source = 0;
            return G_SOURCE_REMOVE;
        }

        if gtk_widget_get_visible((*switcher).panel) == 0 {
            (*switcher).preview_card_build_source = 0;
            return G_SOURCE_REMOVE;
        }

        let start = (*switcher).preview_card_next_index;
        let end = (*(*switcher).previews)
            .len
            .min(start.saturating_add(PREVIEW_CARD_BUILD_BATCH_SIZE));
        if start == 0 {
            clear_grid(switcher);
        }
        let filter = current_search_filter(switcher);
        for i in start..end {
            let preview = ptr_array_index((*switcher).previews, i);
            if i >= (*(*switcher).preview_cards).len {
                let card = create_channel_card(
                    switcher,
                    preview,
                    (*switcher).preview_card_width,
                    (*switcher).preview_width,
                    (*switcher).preview_height,
                );
                g_ptr_array_add(
                    (*switcher).preview_cards,
                    g_object_ref_sink(card as *mut c_void),
                );
            }
            if preview_matches_filter(preview, filter) {
                let visible_count = (*switcher).preview_card_visible_count;
                let columns = (*switcher).preview_card_columns;
                gtk_grid_attach(
                    (*switcher).grid as *mut GtkGrid,
                    ptr_array_index((*switcher).preview_cards, i),
                    (visible_count % columns) as c_int,
                    (visible_count / columns) as c_int,
                    1,
                    1,
                );
                (*switcher).preview_card_visible_count += 1;
            }
        }
        (*switcher).preview_card_next_index = end;

        if end < (*(*switcher).previews).len {
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

        if (*switcher).preview_cards.is_null() {
            (*switcher).preview_cards =
                g_ptr_array_new_with_free_func(Some(g_object_unref_destroy));
        }

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
        remove_source_if_active(&mut (*switcher).preview_card_build_source);
        if (*switcher).previews.is_null() || (*(*switcher).previews).len == 0 {
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
