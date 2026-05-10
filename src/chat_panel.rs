use std::ffi::{c_char, c_double, c_int, c_uint, c_void, CStr};
use std::ptr;

use crate::chat_assets::{
    chat_assets_free, chat_assets_insert_message_text, chat_assets_new, ChatAssets,
};
use crate::twitch_chat::{
    twitch_chat_client_free, twitch_chat_client_new, twitch_chat_client_start, TwitchChatClient,
    TwitchChatLine,
};

const MAX_CHAT_LINES: c_uint = 200;
const CHAT_UI_PRIORITY: c_int = 300;
const G_SOURCE_REMOVE: c_int = 0;
const GDK_EVENT_PROPAGATE: c_int = 0;
const GTK_EVENT_CONTROLLER_SCROLL_VERTICAL: c_int = 1;
const GTK_ORIENTATION_VERTICAL: c_int = 1;
const GTK_POLICY_AUTOMATIC: c_int = 1;
const GTK_POLICY_NEVER: c_int = 2;
const GTK_WRAP_WORD_CHAR: c_int = 3;
const PANGO_WEIGHT_BOLD: c_int = 700;

static FALLBACK_COLORS: [&[u8]; 10] = [
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

pub struct ChatPanel {
    widget: *mut GtkWidget,
    client: *mut TwitchChatClient,
    priv_: *mut ChatPanelPrivate,
}

pub struct ChatPanelPrivate {
    scroller: *mut GtkWidget,
    view: *mut GtkWidget,
    buffer: *mut GtkTextBuffer,
    username_tags: *mut GHashTable,
    assets: *mut ChatAssets,
    reply_tag: *mut GtkTextTag,
    scroll_source: c_uint,
    scroll_state_source: c_uint,
    line_count: c_uint,
    follow_tail: c_int,
    closing: c_int,
}

#[repr(C)]
pub struct GHashTable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GSource {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GdkRGBA {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
}

#[repr(C)]
pub struct GtkAdjustment {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkBox {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEventController {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkEventControllerScroll {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkScrolledWindow {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextBuffer {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GtkTextIter {
    dummy1: *mut c_void,
    dummy2: *mut c_void,
    dummy3: c_int,
    dummy4: c_int,
    dummy5: c_int,
    dummy6: c_int,
    dummy7: c_int,
    dummy8: c_int,
    dummy9: *mut c_void,
    dummy10: *mut c_void,
    dummy11: c_int,
    dummy12: c_int,
    dummy13: c_int,
    dummy14: *mut c_void,
}

#[repr(C)]
pub struct GtkTextTag {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkTextView {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkWidget {
    _private: [u8; 0],
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

unsafe extern "C" {
    fn g_free(mem: *mut c_void);
    fn g_hash_table_destroy(hash_table: *mut GHashTable);
    fn g_hash_table_insert(
        hash_table: *mut GHashTable,
        key: *mut c_void,
        value: *mut c_void,
    ) -> c_int;
    fn g_hash_table_lookup(hash_table: *mut GHashTable, key: *const c_void) -> *mut c_void;
    fn g_hash_table_new_full(
        hash_func: Option<unsafe extern "C" fn(*const c_void) -> c_uint>,
        key_equal_func: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
        key_destroy_func: Option<GDestroyNotify>,
        value_destroy_func: Option<GDestroyNotify>,
    ) -> *mut GHashTable;
    fn g_hash_table_size(hash_table: *mut GHashTable) -> c_uint;
    fn g_idle_add_full(
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    ) -> c_uint;
    fn g_main_context_find_source_by_id(context: *mut c_void, source_id: c_uint) -> *mut GSource;
    fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: *const c_void,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> usize;
    fn g_source_destroy(source: *mut GSource);
    fn g_str_equal(v1: *const c_void, v2: *const c_void) -> c_int;
    fn g_str_hash(v: *const c_void) -> c_uint;
    fn g_strdup(str: *const c_char) -> *mut c_char;

    fn gdk_rgba_parse(rgba: *mut GdkRGBA, spec: *const c_char) -> c_int;
    fn gtk_adjustment_get_page_size(adjustment: *mut GtkAdjustment) -> c_double;
    fn gtk_adjustment_get_upper(adjustment: *mut GtkAdjustment) -> c_double;
    fn gtk_adjustment_get_value(adjustment: *mut GtkAdjustment) -> c_double;
    fn gtk_adjustment_set_value(adjustment: *mut GtkAdjustment, value: c_double);
    fn gtk_box_append(box_: *mut GtkBox, child: *mut GtkWidget);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    fn gtk_event_controller_scroll_new(flags: c_int) -> *mut GtkEventController;
    fn gtk_scrolled_window_get_vadjustment(
        scrolled_window: *mut GtkScrolledWindow,
    ) -> *mut GtkAdjustment;
    fn gtk_scrolled_window_new() -> *mut GtkWidget;
    fn gtk_scrolled_window_set_child(
        scrolled_window: *mut GtkScrolledWindow,
        child: *mut GtkWidget,
    );
    fn gtk_scrolled_window_set_policy(
        scrolled_window: *mut GtkScrolledWindow,
        hscrollbar_policy: c_int,
        vscrollbar_policy: c_int,
    );
    fn gtk_text_buffer_create_tag(
        buffer: *mut GtkTextBuffer,
        tag_name: *const c_char,
        first_property_name: *const c_char,
        ...
    ) -> *mut GtkTextTag;
    fn gtk_text_buffer_delete(
        buffer: *mut GtkTextBuffer,
        start: *mut GtkTextIter,
        end: *mut GtkTextIter,
    );
    fn gtk_text_buffer_get_end_iter(buffer: *mut GtkTextBuffer, iter: *mut GtkTextIter);
    fn gtk_text_buffer_get_start_iter(buffer: *mut GtkTextBuffer, iter: *mut GtkTextIter);
    fn gtk_text_buffer_insert(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        text: *const c_char,
        len: c_int,
    );
    fn gtk_text_buffer_insert_with_tags(
        buffer: *mut GtkTextBuffer,
        iter: *mut GtkTextIter,
        text: *const c_char,
        len: c_int,
        first_tag: *mut GtkTextTag,
        ...
    );
    fn gtk_text_buffer_set_text(buffer: *mut GtkTextBuffer, text: *const c_char, len: c_int);
    fn gtk_text_iter_forward_line(iter: *mut GtkTextIter) -> c_int;
    fn gtk_text_view_get_buffer(text_view: *mut GtkTextView) -> *mut GtkTextBuffer;
    fn gtk_text_view_new() -> *mut GtkWidget;
    fn gtk_text_view_set_bottom_margin(text_view: *mut GtkTextView, bottom_margin: c_int);
    fn gtk_text_view_set_cursor_visible(text_view: *mut GtkTextView, setting: c_int);
    fn gtk_text_view_set_editable(text_view: *mut GtkTextView, setting: c_int);
    fn gtk_text_view_set_left_margin(text_view: *mut GtkTextView, left_margin: c_int);
    fn gtk_text_view_set_right_margin(text_view: *mut GtkTextView, right_margin: c_int);
    fn gtk_text_view_set_top_margin(text_view: *mut GtkTextView, top_margin: c_int);
    fn gtk_text_view_set_wrap_mode(text_view: *mut GtkTextView, wrap_mode: c_int);
    fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: *mut GtkEventController);
    fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    fn gtk_widget_set_focusable(widget: *mut GtkWidget, focusable: c_int);
    fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);

}

unsafe fn is_nonempty(value: *const c_char) -> bool {
    !value.is_null() && *value != 0
}

unsafe fn fallback_username_color(name: *const c_char) -> *const c_char {
    let empty = b"\0".as_ptr() as *const c_char;
    let key = if name.is_null() { empty } else { name };
    let index = (g_str_hash(key as *const c_void) as usize) % FALLBACK_COLORS.len();

    FALLBACK_COLORS[index].as_ptr() as *const c_char
}

unsafe fn get_username_tag(
    panel: *mut ChatPanel,
    name: *const c_char,
    color: *const c_char,
) -> *mut GtkTextTag {
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

    let tag =
        g_hash_table_lookup((*priv_).username_tags, tag_color as *const c_void) as *mut GtkTextTag;
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
        b"foreground\0".as_ptr() as *const c_char,
        tag_color,
        b"weight\0".as_ptr() as *const c_char,
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

unsafe fn trim_old_lines(panel: *mut ChatPanel) {
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

unsafe fn adjustment_is_at_bottom(adjustment: *mut GtkAdjustment) -> bool {
    if adjustment.is_null() {
        return true;
    }

    let value = gtk_adjustment_get_value(adjustment);
    let upper = gtk_adjustment_get_upper(adjustment);
    let page_size = gtk_adjustment_get_page_size(adjustment);

    value + page_size >= upper - 2.0
}

unsafe fn remove_source_if_active(source_id: *mut c_uint) {
    if *source_id == 0 {
        return;
    }

    let source = g_main_context_find_source_by_id(ptr::null_mut(), *source_id);
    if !source.is_null() {
        g_source_destroy(source);
    }
    *source_id = 0;
}

unsafe fn is_scrolled_to_bottom(panel: *mut ChatPanel) -> bool {
    let priv_ = (*panel).priv_;
    let adjustment =
        gtk_scrolled_window_get_vadjustment((*priv_).scroller as *mut GtkScrolledWindow);

    adjustment_is_at_bottom(adjustment)
}

unsafe extern "C" fn scroll_to_end_idle(user_data: *mut c_void) -> c_int {
    let panel = user_data as *mut ChatPanel;
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

unsafe fn queue_scroll_to_end(panel: *mut ChatPanel) {
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

unsafe extern "C" fn update_scroll_state_idle(user_data: *mut c_void) -> c_int {
    let panel = user_data as *mut ChatPanel;
    let priv_ = (*panel).priv_;

    (*priv_).scroll_state_source = 0;

    if (*priv_).closing == 0 {
        (*priv_).follow_tail = if is_scrolled_to_bottom(panel) { 1 } else { 0 };
    }

    G_SOURCE_REMOVE
}

unsafe fn queue_scroll_state_update(panel: *mut ChatPanel) {
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

unsafe extern "C" fn on_chat_scroll(
    _controller: *mut GtkEventControllerScroll,
    _dx: c_double,
    dy: c_double,
    user_data: *mut c_void,
) -> c_int {
    let panel = user_data as *mut ChatPanel;
    let priv_ = (*panel).priv_;

    if dy < 0.0 {
        (*priv_).follow_tail = 0;
    } else if dy > 0.0 {
        queue_scroll_state_update(panel);
    }

    GDK_EVENT_PROPAGATE
}

unsafe extern "C" fn on_chat_adjustment_changed(
    _adjustment: *mut GtkAdjustment,
    user_data: *mut c_void,
) {
    let panel = user_data as *mut ChatPanel;

    if (*(*panel).priv_).follow_tail != 0 {
        queue_scroll_to_end(panel);
    }
}

unsafe fn insert_reply(panel: *mut ChatPanel, iter: *mut GtkTextIter, line: *const TwitchChatLine) {
    let priv_ = (*panel).priv_;

    if !is_nonempty((*line).reply_display_name) {
        return;
    }

    gtk_text_buffer_insert_with_tags(
        (*priv_).buffer,
        iter,
        b"Replying to @\0".as_ptr() as *const c_char,
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
            b": \0".as_ptr() as *const c_char,
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

    gtk_text_buffer_insert((*priv_).buffer, iter, b"\n\0".as_ptr() as *const c_char, -1);
    (*priv_).line_count += 1;
}

unsafe fn append_status_line(panel: *mut ChatPanel, line: *const c_char) {
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
    gtk_text_buffer_insert(
        (*priv_).buffer,
        &mut end,
        b"\n\0".as_ptr() as *const c_char,
        -1,
    );
    (*priv_).line_count += 1;

    trim_old_lines(panel);
    if stick_to_bottom {
        queue_scroll_to_end(panel);
    }
}

unsafe fn append_message(panel: *mut ChatPanel, line: *const TwitchChatLine) {
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
    gtk_text_buffer_insert(
        (*priv_).buffer,
        &mut end,
        b": \0".as_ptr() as *const c_char,
        -1,
    );
    chat_assets_insert_message_text(
        (*priv_).assets,
        (*priv_).buffer,
        (*priv_).view as *mut GtkTextView,
        &mut end,
        (*line).message,
        (*line).emotes,
    );
    gtk_text_buffer_insert(
        (*priv_).buffer,
        &mut end,
        b"\n\0".as_ptr() as *const c_char,
        -1,
    );
    (*priv_).line_count += 1;

    trim_old_lines(panel);
    if stick_to_bottom {
        queue_scroll_to_end(panel);
    }
}

unsafe fn clear_chat(panel: *mut ChatPanel, channel: *const c_char) {
    let priv_ = (*panel).priv_;
    gtk_text_buffer_set_text((*priv_).buffer, b"\0".as_ptr() as *const c_char, -1);
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

unsafe extern "C" fn on_chat_line(line: *const TwitchChatLine, user_data: *mut c_void) {
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

pub unsafe fn chat_panel_new(width: c_int) -> *mut ChatPanel {
    let priv_ = Box::into_raw(Box::new(ChatPanelPrivate {
        scroller: ptr::null_mut(),
        view: ptr::null_mut(),
        buffer: ptr::null_mut(),
        username_tags: ptr::null_mut(),
        assets: ptr::null_mut(),
        reply_tag: ptr::null_mut(),
        scroll_source: 0,
        scroll_state_source: 0,
        line_count: 0,
        follow_tail: 0,
        closing: 0,
    }));
    let panel = Box::into_raw(Box::new(ChatPanel {
        widget: ptr::null_mut(),
        client: ptr::null_mut(),
        priv_,
    }));

    (*panel).widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class((*panel).widget, b"chat-panel\0".as_ptr() as *const c_char);
    gtk_widget_set_size_request((*panel).widget, width, -1);
    gtk_widget_set_vexpand((*panel).widget, 1);

    (*priv_).scroller = gtk_scrolled_window_new();
    gtk_widget_add_css_class(
        (*priv_).scroller,
        b"chat-scroll\0".as_ptr() as *const c_char,
    );
    gtk_scrolled_window_set_policy(
        (*priv_).scroller as *mut GtkScrolledWindow,
        GTK_POLICY_NEVER,
        GTK_POLICY_AUTOMATIC,
    );
    gtk_widget_set_hexpand((*priv_).scroller, 1);
    gtk_widget_set_vexpand((*priv_).scroller, 1);
    gtk_box_append((*panel).widget as *mut GtkBox, (*priv_).scroller);

    let scroll_controller = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
    g_signal_connect_data(
        scroll_controller as *mut c_void,
        b"scroll\0".as_ptr() as *const c_char,
        on_chat_scroll as *const c_void,
        panel as *mut c_void,
        ptr::null_mut(),
        0,
    );
    gtk_widget_add_controller((*priv_).scroller, scroll_controller);

    (*priv_).view = gtk_text_view_new();
    gtk_widget_add_css_class((*priv_).view, b"chat-view\0".as_ptr() as *const c_char);
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
    (*priv_).username_tags =
        g_hash_table_new_full(Some(g_str_hash), Some(g_str_equal), Some(g_free), None);
    (*priv_).assets = chat_assets_new();
    (*priv_).follow_tail = 1;
    (*priv_).reply_tag = gtk_text_buffer_create_tag(
        (*priv_).buffer,
        b"reply\0".as_ptr() as *const c_char,
        b"foreground\0".as_ptr() as *const c_char,
        b"#adadb8\0".as_ptr() as *const c_char,
        b"scale\0".as_ptr() as *const c_char,
        0.90f64,
        ptr::null::<c_char>(),
    );
    gtk_text_buffer_set_text(
        (*priv_).buffer,
        b"No chat connected\0".as_ptr() as *const c_char,
        -1,
    );

    let adjustment =
        gtk_scrolled_window_get_vadjustment((*priv_).scroller as *mut GtkScrolledWindow);
    if !adjustment.is_null() {
        g_signal_connect_data(
            adjustment as *mut c_void,
            b"changed\0".as_ptr() as *const c_char,
            on_chat_adjustment_changed as *const c_void,
            panel as *mut c_void,
            ptr::null_mut(),
            0,
        );
    }

    (*panel).client = twitch_chat_client_new(Some(on_chat_line), panel as *mut c_void);

    panel
}

pub unsafe fn chat_panel_get_widget<W>(panel: *mut ChatPanel) -> *mut W {
    if panel.is_null() {
        return ptr::null_mut();
    }

    (*panel).widget as *mut W
}

pub unsafe fn chat_panel_start(panel: *mut ChatPanel, channel: *const c_char) {
    if panel.is_null() || channel.is_null() || *channel == 0 {
        return;
    }

    clear_chat(panel, channel);
    twitch_chat_client_start((*panel).client, channel);
}

pub unsafe fn chat_panel_free(panel: *mut ChatPanel) {
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

        remove_source_if_active(&mut (*priv_).scroll_source);
        remove_source_if_active(&mut (*priv_).scroll_state_source);

        if !(*priv_).username_tags.is_null() {
            g_hash_table_destroy((*priv_).username_tags);
        }
        if !(*priv_).assets.is_null() {
            chat_assets_free((*priv_).assets);
        }

        drop(Box::from_raw(priv_));
    }

    drop(Box::from_raw(panel));
}

pub unsafe fn chat_panel_test_fallback_username_color(name: *const c_char) -> *const c_char {
    fallback_username_color(name)
}

pub unsafe fn chat_panel_test_adjustment_is_at_bottom(adjustment: *mut GtkAdjustment) -> c_int {
    if adjustment_is_at_bottom(adjustment) {
        1
    } else {
        0
    }
}
