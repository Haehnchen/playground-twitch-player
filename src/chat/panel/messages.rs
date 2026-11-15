use super::ffi::{
    G_SOURCE_REMOVE, GDK_EVENT_PROPAGATE, GdkRGBA, GtkAdjustment, GtkEventControllerScroll,
    GtkScrolledWindow, GtkTextIter, GtkTextTag, GtkTextView, PANGO_WEIGHT_BOLD,
    g_hash_table_insert, g_hash_table_lookup, g_hash_table_size, g_idle_add_full, g_str_hash,
    g_strdup, gdk_rgba_parse, gtk_adjustment_get_page_size, gtk_adjustment_get_upper,
    gtk_adjustment_get_value, gtk_adjustment_set_value, gtk_scrolled_window_get_vadjustment,
    gtk_text_buffer_create_tag, gtk_text_buffer_delete, gtk_text_buffer_get_end_iter,
    gtk_text_buffer_get_start_iter, gtk_text_buffer_insert, gtk_text_buffer_insert_with_tags,
    gtk_text_buffer_set_text, gtk_text_iter_forward_line,
};
use super::{ChatPanel, remove_source_if_active};
use crate::chat::assets::chat_assets_insert_message_text;
use crate::glib_strings::is_nonempty;
use crate::twitch::chat::TwitchChatLine;
use std::ffi::{CStr, c_char, c_double, c_int, c_uint, c_void};
use std::ptr;

pub(super) const MAX_CHAT_LINES: c_uint = 200;

pub(super) static FALLBACK_COLORS: [&[u8]; 10] = [
    b"#ff7f50\0",
    b"#9acd32\0",
    b"#1e90ff\0",
    b"#ff69b4\0",
    b"#ba55d3\0",
    b"#00b5ad\0",
    b"#f2c94c\0",
    b"#7aa2ff\0",
    b"#ff8a65\0",
    b"#57d68d\0",
];

pub(super) unsafe fn fallback_username_color(name: *const c_char) -> *const c_char {
    unsafe {
        let empty = c"".as_ptr();
        let key = if name.is_null() { empty } else { name };
        let index = (g_str_hash(key as *const c_void) as usize) % FALLBACK_COLORS.len();

        FALLBACK_COLORS[index].as_ptr() as *const c_char
    }
}

pub(super) unsafe fn get_username_tag(
    panel: *mut ChatPanel,
    name: *const c_char,
    color: *const c_char,
) -> *mut GtkTextTag {
    unsafe {
        let priv_ = (*panel).priv_;
        let mut parsed = GdkRGBA {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.0,
        };
        let tag_color = if !is_nonempty(color) || gdk_rgba_parse(&mut parsed, color) == 0 {
            fallback_username_color(name)
        } else {
            color
        };

        let tag = g_hash_table_lookup((*priv_).username_tags, tag_color as *const c_void)
            as *mut GtkTextTag;
        if !tag.is_null() {
            return tag;
        }

        let tag_name = format!(
            "username-color-{}\0",
            g_hash_table_size((*priv_).username_tags)
        );
        let tag = gtk_text_buffer_create_tag(
            (*priv_).buffer,
            tag_name.as_ptr() as *const c_char,
            c"foreground".as_ptr(),
            tag_color,
            c"weight".as_ptr(),
            PANGO_WEIGHT_BOLD,
            ptr::null::<c_char>(),
        );
        g_hash_table_insert(
            (*priv_).username_tags,
            g_strdup(tag_color) as *mut c_void,
            tag as *mut c_void,
        );
        tag
    }
}

pub(super) unsafe fn trim_old_lines(panel: *mut ChatPanel) {
    unsafe {
        let priv_ = (*panel).priv_;

        while (*priv_).line_count > MAX_CHAT_LINES {
            let mut start = GtkTextIter {
                dummy1: ptr::null_mut(),
                dummy2: ptr::null_mut(),
                dummy3: 0,
                dummy4: 0,
                dummy5: 0,
                dummy6: 0,
                dummy7: 0,
                dummy8: 0,
                dummy9: ptr::null_mut(),
                dummy10: ptr::null_mut(),
                dummy11: 0,
                dummy12: 0,
                dummy13: 0,
                dummy14: ptr::null_mut(),
            };
            gtk_text_buffer_get_start_iter((*priv_).buffer, &mut start);

            let mut delete_end = start;
            gtk_text_iter_forward_line(&mut delete_end);
            gtk_text_buffer_delete((*priv_).buffer, &mut start, &mut delete_end);
            (*priv_).line_count -= 1;
        }
    }
}

pub(super) unsafe fn insert_reply(
    panel: *mut ChatPanel,
    iter: *mut GtkTextIter,
    line: *const TwitchChatLine,
) {
    unsafe {
        let priv_ = (*panel).priv_;

        if !is_nonempty((*line).reply_display_name) {
            return;
        }

        gtk_text_buffer_insert_with_tags(
            (*priv_).buffer,
            iter,
            c"Replying to @".as_ptr(),
            -1,
            (*priv_).reply_tag,
            ptr::null_mut::<GtkTextTag>(),
        );
        gtk_text_buffer_insert_with_tags(
            (*priv_).buffer,
            iter,
            (*line).reply_display_name,
            -1,
            (*priv_).reply_tag,
            ptr::null_mut::<GtkTextTag>(),
        );

        if is_nonempty((*line).reply_message) {
            gtk_text_buffer_insert_with_tags(
                (*priv_).buffer,
                iter,
                c": ".as_ptr(),
                -1,
                (*priv_).reply_tag,
                ptr::null_mut::<GtkTextTag>(),
            );
            gtk_text_buffer_insert_with_tags(
                (*priv_).buffer,
                iter,
                (*line).reply_message,
                -1,
                (*priv_).reply_tag,
                ptr::null_mut::<GtkTextTag>(),
            );
        }

        gtk_text_buffer_insert((*priv_).buffer, iter, c"\n".as_ptr(), -1);
        (*priv_).line_count += 1;
    }
}

pub(super) unsafe fn append_status_line(panel: *mut ChatPanel, line: *const c_char) {
    unsafe {
        let priv_ = (*panel).priv_;

        if (*priv_).closing != 0 {
            return;
        }

        let stick_to_bottom = (*priv_).follow_tail != 0 || is_scrolled_to_bottom(panel);
        let mut end = GtkTextIter {
            dummy1: ptr::null_mut(),
            dummy2: ptr::null_mut(),
            dummy3: 0,
            dummy4: 0,
            dummy5: 0,
            dummy6: 0,
            dummy7: 0,
            dummy8: 0,
            dummy9: ptr::null_mut(),
            dummy10: ptr::null_mut(),
            dummy11: 0,
            dummy12: 0,
            dummy13: 0,
            dummy14: ptr::null_mut(),
        };

        gtk_text_buffer_get_end_iter((*priv_).buffer, &mut end);
        gtk_text_buffer_insert((*priv_).buffer, &mut end, line, -1);
        gtk_text_buffer_insert((*priv_).buffer, &mut end, c"\n".as_ptr(), -1);
        (*priv_).line_count += 1;

        trim_old_lines(panel);
        if stick_to_bottom {
            queue_scroll_to_end(panel);
        }
    }
}

// Append a message without pulling readers away from older messages.
pub(super) unsafe fn append_message(panel: *mut ChatPanel, line: *const TwitchChatLine) {
    unsafe {
        let priv_ = (*panel).priv_;

        if (*priv_).closing != 0 {
            return;
        }

        let stick_to_bottom = (*priv_).follow_tail != 0 || is_scrolled_to_bottom(panel);
        let mut end = GtkTextIter {
            dummy1: ptr::null_mut(),
            dummy2: ptr::null_mut(),
            dummy3: 0,
            dummy4: 0,
            dummy5: 0,
            dummy6: 0,
            dummy7: 0,
            dummy8: 0,
            dummy9: ptr::null_mut(),
            dummy10: ptr::null_mut(),
            dummy11: 0,
            dummy12: 0,
            dummy13: 0,
            dummy14: ptr::null_mut(),
        };

        gtk_text_buffer_get_end_iter((*priv_).buffer, &mut end);

        insert_reply(panel, &mut end, line);

        let username_tag = get_username_tag(panel, (*line).display_name, (*line).color);
        gtk_text_buffer_insert_with_tags(
            (*priv_).buffer,
            &mut end,
            (*line).display_name,
            -1,
            username_tag,
            ptr::null_mut::<GtkTextTag>(),
        );
        gtk_text_buffer_insert((*priv_).buffer, &mut end, c": ".as_ptr(), -1);
        chat_assets_insert_message_text(
            (*priv_).assets,
            (*priv_).buffer,
            (*priv_).view as *mut GtkTextView,
            &mut end,
            (*line).message,
            (*line).emotes,
        );
        gtk_text_buffer_insert((*priv_).buffer, &mut end, c"\n".as_ptr(), -1);
        (*priv_).line_count += 1;

        trim_old_lines(panel);
        if stick_to_bottom {
            queue_scroll_to_end(panel);
        }
    }
}

pub(super) unsafe fn clear_chat(panel: *mut ChatPanel, channel: *const c_char) {
    unsafe {
        let priv_ = (*panel).priv_;
        gtk_text_buffer_set_text((*priv_).buffer, c"".as_ptr(), -1);
        (*priv_).line_count = 0;
        (*priv_).follow_tail = 1;

        if !channel.is_null() {
            let channel = CStr::from_ptr(channel).to_bytes();
            let mut line = Vec::with_capacity(channel.len() + 20);
            line.extend_from_slice(b"Verbinde mit #");
            line.extend_from_slice(channel);
            line.extend_from_slice(b" ...\0");
            append_status_line(panel, line.as_ptr() as *const c_char);
        }
    }
}

pub(super) unsafe extern "C" fn on_chat_line(line: *const TwitchChatLine, user_data: *mut c_void) {
    unsafe {
        let panel = user_data as *mut ChatPanel;
        let priv_ = (*panel).priv_;

        if (*priv_).closing == 0 {
            if (*line).kind == 1 {
                append_message(panel, line);
            } else {
                append_status_line(panel, (*line).message);
            }
        }
    }
}

pub(super) const CHAT_UI_PRIORITY: c_int = 300;

pub(super) fn scroll_is_at_bottom(value: f64, upper: f64, page_size: f64) -> bool {
    value + page_size >= upper - 2.0
}

pub(super) unsafe fn adjustment_is_at_bottom(adjustment: *mut GtkAdjustment) -> bool {
    unsafe {
        if adjustment.is_null() {
            return true;
        }

        let value = gtk_adjustment_get_value(adjustment);
        let upper = gtk_adjustment_get_upper(adjustment);
        let page_size = gtk_adjustment_get_page_size(adjustment);

        scroll_is_at_bottom(value, upper, page_size)
    }
}

pub(super) unsafe fn is_scrolled_to_bottom(panel: *mut ChatPanel) -> bool {
    unsafe {
        let priv_ = (*panel).priv_;
        let adjustment =
            gtk_scrolled_window_get_vadjustment((*priv_).scroller as *mut GtkScrolledWindow);

        adjustment_is_at_bottom(adjustment)
    }
}

pub(super) unsafe extern "C" fn scroll_to_end_idle(user_data: *mut c_void) -> c_int {
    unsafe {
        let panel = user_data as *mut ChatPanel;
        if panel.is_null() || (*panel).priv_.is_null() {
            return G_SOURCE_REMOVE;
        }

        let priv_ = (*panel).priv_;

        (*priv_).scroll_source = 0;

        if (*priv_).closing != 0 {
            return G_SOURCE_REMOVE;
        }

        let adjustment =
            gtk_scrolled_window_get_vadjustment((*priv_).scroller as *mut GtkScrolledWindow);
        if !adjustment.is_null() {
            let upper = gtk_adjustment_get_upper(adjustment);
            let page_size = gtk_adjustment_get_page_size(adjustment);
            gtk_adjustment_set_value(adjustment, (upper - page_size).max(0.0));
        }

        (*priv_).follow_tail = 1;

        G_SOURCE_REMOVE
    }
}

// Scroll after GTK has updated the text layout.
pub(super) unsafe fn queue_scroll_to_end(panel: *mut ChatPanel) {
    unsafe {
        let priv_ = (*panel).priv_;

        (*priv_).follow_tail = 1;

        remove_source_if_active(&mut (*priv_).scroll_source);

        (*priv_).scroll_source = g_idle_add_full(
            CHAT_UI_PRIORITY,
            Some(scroll_to_end_idle),
            panel as *mut c_void,
            None,
        );
    }
}

pub(super) unsafe extern "C" fn update_scroll_state_idle(user_data: *mut c_void) -> c_int {
    unsafe {
        let panel = user_data as *mut ChatPanel;
        if panel.is_null() || (*panel).priv_.is_null() {
            return G_SOURCE_REMOVE;
        }

        let priv_ = (*panel).priv_;

        (*priv_).scroll_state_source = 0;

        if (*priv_).closing == 0 {
            (*priv_).follow_tail = if is_scrolled_to_bottom(panel) { 1 } else { 0 };
        }

        G_SOURCE_REMOVE
    }
}

pub(super) unsafe fn queue_scroll_state_update(panel: *mut ChatPanel) {
    unsafe {
        let priv_ = (*panel).priv_;

        if (*priv_).scroll_state_source == 0 {
            (*priv_).scroll_state_source = g_idle_add_full(
                CHAT_UI_PRIORITY,
                Some(update_scroll_state_idle),
                panel as *mut c_void,
                None,
            );
        }
    }
}

pub(super) unsafe extern "C" fn on_chat_scroll(
    _controller: *mut GtkEventControllerScroll,
    _dx: c_double,
    dy: c_double,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        let panel = user_data as *mut ChatPanel;
        if panel.is_null() || (*panel).priv_.is_null() {
            return GDK_EVENT_PROPAGATE;
        }

        let priv_ = (*panel).priv_;

        if dy < 0.0 {
            (*priv_).follow_tail = 0;
        } else if dy > 0.0 {
            queue_scroll_state_update(panel);
        }

        GDK_EVENT_PROPAGATE
    }
}

pub(super) unsafe extern "C" fn on_chat_adjustment_changed(
    _adjustment: *mut GtkAdjustment,
    user_data: *mut c_void,
) {
    unsafe {
        let panel = user_data as *mut ChatPanel;
        if panel.is_null() || (*panel).priv_.is_null() {
            return;
        }

        let priv_ = (*panel).priv_;
        if (*priv_).closing == 0 && (*priv_).follow_tail != 0 {
            queue_scroll_to_end(panel);
        }
    }
}
