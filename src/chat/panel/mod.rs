mod ffi;
mod messages;

pub use self::ffi::{
    GSource, GdkRGBA, GtkAdjustment, GtkBox, GtkEventController, GtkEventControllerScroll,
    GtkScrolledWindow, GtkTextBuffer, GtkTextIter, GtkTextTag, GtkTextView, GtkWidget,
};

use self::ffi::{
    GTK_EVENT_CONTROLLER_SCROLL_VERTICAL, GTK_ORIENTATION_VERTICAL, GTK_POLICY_AUTOMATIC,
    GTK_POLICY_NEVER, GTK_WRAP_WORD_CHAR, g_main_context_find_source_by_id, g_object_ref_sink,
    g_object_unref, g_signal_connect_data, g_signal_handler_disconnect, g_source_destroy,
    gtk_box_append, gtk_box_new, gtk_event_controller_scroll_new,
    gtk_scrolled_window_get_vadjustment, gtk_scrolled_window_new, gtk_scrolled_window_set_child,
    gtk_scrolled_window_set_policy, gtk_text_buffer_create_tag, gtk_text_buffer_set_text,
    gtk_text_view_get_buffer, gtk_text_view_new, gtk_text_view_set_bottom_margin,
    gtk_text_view_set_cursor_visible, gtk_text_view_set_editable, gtk_text_view_set_left_margin,
    gtk_text_view_set_right_margin, gtk_text_view_set_top_margin, gtk_text_view_set_wrap_mode,
    gtk_widget_add_controller, gtk_widget_add_css_class, gtk_widget_set_focusable,
    gtk_widget_set_hexpand, gtk_widget_set_size_request, gtk_widget_set_vexpand,
};
use self::messages::{clear_chat, on_chat_line};
use self::messages::{on_chat_adjustment_changed, on_chat_scroll};
use crate::chat::assets::{ChatAssets, chat_assets_free, chat_assets_new};
use crate::twitch::chat::{
    TwitchChatClient, twitch_chat_client_free, twitch_chat_client_new, twitch_chat_client_start,
};
use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

pub struct ChatPanel {
    widget: *mut GtkWidget,
    client: *mut TwitchChatClient,
    priv_: *mut ChatPanelPrivate,
}

#[derive(Default)]
pub struct ChatPanelPrivate {
    scroller: *mut GtkWidget,
    scroll_controller: *mut GtkEventController,
    view: *mut GtkWidget,
    buffer: *mut GtkTextBuffer,
    adjustment: *mut GtkAdjustment,

    username_tags: HashMap<Vec<u8>, *mut GtkTextTag>,
    username_tag_order: VecDeque<Vec<u8>>,
    assets: *mut ChatAssets,
    reply_tag: *mut GtkTextTag,

    scroll_handler: usize,
    adjustment_changed_handler: usize,
    scroll_source: c_uint,
    scroll_state_source: c_uint,

    follow_tail: c_int,
    closing: c_int,
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    unsafe {
        if *source_id == 0 {
            return;
        }

        let source = g_main_context_find_source_by_id(ptr::null_mut(), *source_id);
        if !source.is_null() {
            g_source_destroy(source);
        }
        *source_id = 0;
    }
}

unsafe fn disconnect_signal_handler(instance: *mut c_void, handler_id: *mut usize) {
    unsafe {
        if instance.is_null() || *handler_id == 0 {
            return;
        }

        g_signal_handler_disconnect(instance, *handler_id);
        *handler_id = 0;
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
pub unsafe fn chat_panel_new(width: c_int) -> *mut ChatPanel {
    unsafe {
        let priv_ = Box::into_raw(Box::new(ChatPanelPrivate::default()));
        let panel = Box::into_raw(Box::new(ChatPanel {
            widget: ptr::null_mut(),
            client: ptr::null_mut(),
            priv_,
        }));

        (*panel).widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
        g_object_ref_sink((*panel).widget as *mut c_void);
        gtk_widget_add_css_class((*panel).widget, c"chat-panel".as_ptr());
        gtk_widget_set_size_request((*panel).widget, width, -1);
        gtk_widget_set_vexpand((*panel).widget, 1);

        (*priv_).scroller = gtk_scrolled_window_new();
        gtk_widget_add_css_class((*priv_).scroller, c"chat-scroll".as_ptr());
        gtk_scrolled_window_set_policy(
            (*priv_).scroller as *mut GtkScrolledWindow,
            GTK_POLICY_NEVER,
            GTK_POLICY_AUTOMATIC,
        );
        gtk_widget_set_hexpand((*priv_).scroller, 1);
        gtk_widget_set_vexpand((*priv_).scroller, 1);
        gtk_box_append((*panel).widget as *mut GtkBox, (*priv_).scroller);

        let scroll_controller =
            gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
        (*priv_).scroll_controller = scroll_controller;
        (*priv_).scroll_handler = g_signal_connect_data(
            scroll_controller as *mut c_void,
            c"scroll".as_ptr(),
            on_chat_scroll as *const c_void,
            panel as *mut c_void,
            None,
            0,
        );
        gtk_widget_add_controller((*priv_).scroller, scroll_controller);

        (*priv_).view = gtk_text_view_new();
        gtk_widget_add_css_class((*priv_).view, c"chat-view".as_ptr());
        gtk_widget_set_focusable((*priv_).view, 0);
        gtk_text_view_set_editable((*priv_).view as *mut GtkTextView, 0);
        gtk_text_view_set_cursor_visible((*priv_).view as *mut GtkTextView, 0);
        gtk_text_view_set_wrap_mode((*priv_).view as *mut GtkTextView, GTK_WRAP_WORD_CHAR);
        gtk_text_view_set_left_margin((*priv_).view as *mut GtkTextView, 10);
        gtk_text_view_set_right_margin((*priv_).view as *mut GtkTextView, 10);
        gtk_text_view_set_top_margin((*priv_).view as *mut GtkTextView, 8);
        gtk_text_view_set_bottom_margin((*priv_).view as *mut GtkTextView, 8);
        gtk_scrolled_window_set_child((*priv_).scroller as *mut GtkScrolledWindow, (*priv_).view);

        (*priv_).buffer = gtk_text_view_get_buffer((*priv_).view as *mut GtkTextView);
        (*priv_).assets = chat_assets_new();
        (*priv_).follow_tail = 1;
        (*priv_).reply_tag = gtk_text_buffer_create_tag(
            (*priv_).buffer,
            c"reply".as_ptr(),
            c"foreground".as_ptr(),
            c"#adadb8".as_ptr(),
            c"scale".as_ptr(),
            0.90f64,
            ptr::null::<c_char>(),
        );
        gtk_text_buffer_set_text((*priv_).buffer, c"No chat connected".as_ptr(), -1);

        let adjustment =
            gtk_scrolled_window_get_vadjustment((*priv_).scroller as *mut GtkScrolledWindow);
        if !adjustment.is_null() {
            (*priv_).adjustment = adjustment;
            (*priv_).adjustment_changed_handler = g_signal_connect_data(
                adjustment as *mut c_void,
                c"changed".as_ptr(),
                on_chat_adjustment_changed as *const c_void,
                panel as *mut c_void,
                None,
                0,
            );
        }

        (*panel).client = twitch_chat_client_new(Some(on_chat_line), panel as *mut c_void);

        panel
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `panel`: null or live instance.
/// - Result: borrowed GTK widget; compatible `W`; invalid after widget destruction.
pub unsafe fn chat_panel_get_widget<W>(panel: *mut ChatPanel) -> *mut W {
    unsafe {
        if panel.is_null() {
            return ptr::null_mut();
        }

        (*panel).widget as *mut W
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `panel`: null or live instance.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn chat_panel_start(panel: *mut ChatPanel, channel: *const c_char) {
    unsafe {
        if panel.is_null() || channel.is_null() || *channel == 0 {
            return;
        }

        clear_chat(panel, channel);
        twitch_chat_client_start((*panel).client, channel);
    }
}

/// # Safety
/// - GTK initialized; GTK main thread.
/// - `panel`: null or live instance.
/// - Release owner once; no subsequent pointer access.
pub unsafe fn chat_panel_free(panel: *mut ChatPanel) {
    unsafe {
        if panel.is_null() {
            return;
        }

        if !(*panel).priv_.is_null() {
            (*(*panel).priv_).closing = 1;
        }

        twitch_chat_client_free((*panel).client);
        (*panel).client = ptr::null_mut();

        if !(*panel).priv_.is_null() {
            let priv_ = (*panel).priv_;

            disconnect_signal_handler(
                (*priv_).adjustment as *mut c_void,
                &mut (*priv_).adjustment_changed_handler,
            );
            disconnect_signal_handler(
                (*priv_).scroll_controller as *mut c_void,
                &mut (*priv_).scroll_handler,
            );
            remove_source_if_active(&mut (*priv_).scroll_source);
            remove_source_if_active(&mut (*priv_).scroll_state_source);

            if !(*priv_).assets.is_null() {
                chat_assets_free((*priv_).assets);
            }

            (*panel).priv_ = ptr::null_mut();
            drop(Box::from_raw(priv_));
        }

        if !(*panel).widget.is_null() {
            g_object_unref((*panel).widget as *mut c_void);
            (*panel).widget = ptr::null_mut();
        }

        drop(Box::from_raw(panel));
    }
}

#[cfg(test)]
mod tests {
    use super::messages::fallback_username_color;
    use super::messages::{adjustment_is_at_bottom, scroll_is_at_bottom};
    use std::ffi::CStr;
    use std::ptr;

    #[test]
    fn reply_lines_count_toward_the_chat_history_limit() {
        use gtk4::{glib::translate::from_glib_full, prelude::*};
        unsafe {
            let buffer: gtk4::TextBuffer =
                from_glib_full(gtk4::ffi::gtk_text_buffer_new(ptr::null_mut()));
            let text: String = (0..600)
                .map(|i| format!("Reply to parent {i}\nMessage {i}\n"))
                .collect();
            buffer.set_text(&text);
            let mut private = super::ChatPanelPrivate {
                buffer: buffer.as_ptr(),
                ..Default::default()
            };
            let mut panel = super::ChatPanel {
                widget: ptr::null_mut(),
                client: ptr::null_mut(),
                priv_: &mut private,
            };
            super::messages::trim_old_lines(&mut panel);
            assert_eq!(buffer.line_count(), 201);
            let (start, end) = buffer.bounds();
            let retained = buffer.text(&start, &end, false);
            assert!(retained.starts_with("Reply to parent 500\nMessage 500\n"));
            assert!(retained.ends_with("Message 599\n"));
        }
    }

    #[test]
    fn username_colors_are_bounded_and_recent_colors_stay_cached() {
        use gtk4::{glib::translate::from_glib_full, prelude::*};
        use std::ffi::CString;
        unsafe {
            let buffer: gtk4::TextBuffer =
                from_glib_full(gtk4::ffi::gtk_text_buffer_new(ptr::null_mut()));
            let mut private = super::ChatPanelPrivate {
                buffer: buffer.as_ptr(),
                ..Default::default()
            };
            let mut panel = super::ChatPanel {
                widget: ptr::null_mut(),
                client: ptr::null_mut(),
                priv_: &mut private,
            };
            let hot = super::messages::get_username_tag(
                &mut panel,
                c"User".as_ptr(),
                c"#ff00ff".as_ptr(),
            );
            for i in 0..600 {
                let color = CString::new(format!("#{i:06x}")).unwrap();
                super::messages::get_username_tag(&mut panel, c"User".as_ptr(), color.as_ptr());
                assert_eq!(
                    super::messages::get_username_tag(
                        &mut panel,
                        c"User".as_ptr(),
                        c"#ff00ff".as_ptr()
                    ),
                    hot
                );
            }
            assert!(buffer.tag_table().size() <= 256);
            super::messages::clear_chat(&mut panel, ptr::null());
            assert_eq!(buffer.tag_table().size(), 0);
            assert!(private.username_tags.is_empty());
        }
    }

    #[test]
    fn fallback_colors_are_deterministic() {
        unsafe {
            let first = CStr::from_ptr(fallback_username_color(c"alice".as_ptr()));
            let second = CStr::from_ptr(fallback_username_color(c"alice".as_ptr()));
            let empty = CStr::from_ptr(fallback_username_color(ptr::null()));
            assert_eq!(first, second);
            for color in [first, empty] {
                assert_eq!(color.to_bytes().len(), 7);
                assert!(color.to_bytes().starts_with(b"#"));
            }
        }
    }

    #[test]
    fn scroll_bottom_allows_two_pixels_of_tolerance() {
        assert!(scroll_is_at_bottom(90.0, 100.0, 10.0));
        assert!(scroll_is_at_bottom(88.0, 100.0, 10.0));
        assert!(!scroll_is_at_bottom(87.0, 100.0, 10.0));
        assert!(scroll_is_at_bottom(0.0, 0.0, 10.0));
    }

    #[test]
    fn missing_adjustment_counts_as_bottom() {
        unsafe { assert!(adjustment_is_at_bottom(ptr::null_mut())) };
    }
}
