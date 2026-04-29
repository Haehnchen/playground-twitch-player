#define G_LOG_DOMAIN "chat-panel"

#include "chat_assets.h"
#include "chat_panel.h"

#define MAX_CHAT_LINES 200
#define CHAT_UI_PRIORITY G_PRIORITY_LOW

struct ChatPanelPrivate {
    GtkWidget *scroller;
    GtkWidget *view;
    GtkTextBuffer *buffer;
    GHashTable *username_tags;
    ChatAssets *assets;
    GtkTextTag *reply_tag;
    guint scroll_source;
    guint scroll_state_source;
    guint line_count;
    gboolean follow_tail;
    gboolean closing;
};

static const char *fallback_username_color(const char *name)
{
    static const char *colors[] = {
        "#ff7f50",
        "#9acd32",
        "#1e90ff",
        "#ff69b4",
        "#ba55d3",
        "#00b5ad",
        "#f2c94c",
        "#7aa2ff",
        "#ff8a65",
        "#57d68d",
    };

    return colors[g_str_hash(name != NULL ? name : "") % G_N_ELEMENTS(colors)];
}

static GtkTextTag *get_username_tag(ChatPanel *panel, const char *name, const char *color)
{
    ChatPanelPrivate *priv = panel->priv;
    GdkRGBA parsed;
    const char *tag_color = color;

    if (tag_color == NULL || tag_color[0] == '\0' || !gdk_rgba_parse(&parsed, tag_color)) {
        tag_color = fallback_username_color(name);
    }

    GtkTextTag *tag = g_hash_table_lookup(priv->username_tags, tag_color);
    if (tag != NULL) {
        return tag;
    }

    g_autofree char *tag_name = g_strdup_printf("username-color-%u", g_hash_table_size(priv->username_tags));
    tag = gtk_text_buffer_create_tag(
        priv->buffer,
        tag_name,
        "foreground", tag_color,
        "weight", PANGO_WEIGHT_BOLD,
        NULL
    );
    g_hash_table_insert(priv->username_tags, g_strdup(tag_color), tag);
    return tag;
}

static void trim_old_lines(ChatPanel *panel)
{
    ChatPanelPrivate *priv = panel->priv;

    /* Bound memory and widget work for long-running chat sessions. */
    while (priv->line_count > MAX_CHAT_LINES) {
        GtkTextIter start;
        GtkTextIter delete_end;

        gtk_text_buffer_get_start_iter(priv->buffer, &start);
        delete_end = start;
        gtk_text_iter_forward_line(&delete_end);
        gtk_text_buffer_delete(priv->buffer, &start, &delete_end);
        priv->line_count--;
    }
}

static gboolean adjustment_is_at_bottom(GtkAdjustment *adjustment)
{
    if (adjustment == NULL) {
        return TRUE;
    }

    double value = gtk_adjustment_get_value(adjustment);
    double upper = gtk_adjustment_get_upper(adjustment);
    double page_size = gtk_adjustment_get_page_size(adjustment);

    return value + page_size >= upper - 2.0;
}

static void remove_source_if_active(guint *source_id)
{
    if (*source_id == 0) {
        return;
    }

    GSource *source = g_main_context_find_source_by_id(NULL, *source_id);
    if (source != NULL) {
        g_source_destroy(source);
    }
    *source_id = 0;
}

static gboolean is_scrolled_to_bottom(ChatPanel *panel)
{
    ChatPanelPrivate *priv = panel->priv;
    GtkAdjustment *adjustment = gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(priv->scroller));

    return adjustment_is_at_bottom(adjustment);
}

static gboolean scroll_to_end_idle(gpointer user_data)
{
    ChatPanel *panel = user_data;
    ChatPanelPrivate *priv = panel->priv;

    priv->scroll_source = 0;

    if (priv->closing) {
        return G_SOURCE_REMOVE;
    }

    /* GTK computes the final scroll range after layout, so scroll from idle. */
    GtkAdjustment *adjustment = gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(priv->scroller));
    if (adjustment != NULL) {
        double upper = gtk_adjustment_get_upper(adjustment);
        double page_size = gtk_adjustment_get_page_size(adjustment);
        gtk_adjustment_set_value(adjustment, MAX(0.0, upper - page_size));
    }

    priv->follow_tail = TRUE;

    return G_SOURCE_REMOVE;
}

static void queue_scroll_to_end(ChatPanel *panel)
{
    ChatPanelPrivate *priv = panel->priv;

    priv->follow_tail = TRUE;

    remove_source_if_active(&priv->scroll_source);

    priv->scroll_source = g_idle_add_full(CHAT_UI_PRIORITY, scroll_to_end_idle, panel, NULL);
}

static gboolean update_scroll_state_idle(gpointer user_data)
{
    ChatPanel *panel = user_data;
    ChatPanelPrivate *priv = panel->priv;

    priv->scroll_state_source = 0;

    if (!priv->closing) {
        priv->follow_tail = is_scrolled_to_bottom(panel);
    }

    return G_SOURCE_REMOVE;
}

static void queue_scroll_state_update(ChatPanel *panel)
{
    ChatPanelPrivate *priv = panel->priv;

    if (priv->scroll_state_source == 0) {
        priv->scroll_state_source = g_idle_add_full(CHAT_UI_PRIORITY, update_scroll_state_idle, panel, NULL);
    }
}

static gboolean on_chat_scroll(GtkEventControllerScroll *controller, double dx, double dy, gpointer user_data)
{
    (void)controller;
    (void)dx;
    ChatPanel *panel = user_data;
    ChatPanelPrivate *priv = panel->priv;

    if (dy < 0.0) {
        priv->follow_tail = FALSE;
    } else if (dy > 0.0) {
        /* Re-check after GTK applies the wheel delta. */
        queue_scroll_state_update(panel);
    }

    return GDK_EVENT_PROPAGATE;
}

static void on_chat_adjustment_changed(GtkAdjustment *adjustment, gpointer user_data)
{
    (void)adjustment;
    ChatPanel *panel = user_data;

    if (panel->priv->follow_tail) {
        queue_scroll_to_end(panel);
    }
}

static void insert_reply(ChatPanel *panel, GtkTextIter *iter, const TwitchChatLine *line)
{
    if (line->reply_display_name == NULL || line->reply_display_name[0] == '\0') {
        return;
    }

    gtk_text_buffer_insert_with_tags(panel->priv->buffer, iter, "Replying to @", -1, panel->priv->reply_tag, NULL);
    gtk_text_buffer_insert_with_tags(panel->priv->buffer, iter, line->reply_display_name, -1, panel->priv->reply_tag, NULL);

    if (line->reply_message != NULL && line->reply_message[0] != '\0') {
        gtk_text_buffer_insert_with_tags(panel->priv->buffer, iter, ": ", -1, panel->priv->reply_tag, NULL);
        gtk_text_buffer_insert_with_tags(panel->priv->buffer, iter, line->reply_message, -1, panel->priv->reply_tag, NULL);
    }

    gtk_text_buffer_insert(panel->priv->buffer, iter, "\n", -1);
    panel->priv->line_count++;
}

static void append_status_line(ChatPanel *panel, const char *line)
{
    ChatPanelPrivate *priv = panel->priv;
    GtkTextIter end;

    if (priv->closing) {
        return;
    }

    gboolean stick_to_bottom = priv->follow_tail || is_scrolled_to_bottom(panel);

    gtk_text_buffer_get_end_iter(priv->buffer, &end);
    gtk_text_buffer_insert(priv->buffer, &end, line, -1);
    gtk_text_buffer_insert(priv->buffer, &end, "\n", -1);
    priv->line_count++;

    trim_old_lines(panel);
    if (stick_to_bottom) {
        queue_scroll_to_end(panel);
    }
}

static void append_message(ChatPanel *panel, const TwitchChatLine *line)
{
    ChatPanelPrivate *priv = panel->priv;
    GtkTextIter end;

    if (priv->closing) {
        return;
    }

    gboolean stick_to_bottom = priv->follow_tail || is_scrolled_to_bottom(panel);

    gtk_text_buffer_get_end_iter(priv->buffer, &end);

    insert_reply(panel, &end, line);

    GtkTextTag *username_tag = get_username_tag(panel, line->display_name, line->color);
    gtk_text_buffer_insert_with_tags(priv->buffer, &end, line->display_name, -1, username_tag, NULL);
    gtk_text_buffer_insert(priv->buffer, &end, ": ", -1);
    chat_assets_insert_message_text(priv->assets, priv->buffer, GTK_TEXT_VIEW(priv->view), &end, line->message, line->emotes);
    gtk_text_buffer_insert(priv->buffer, &end, "\n", -1);
    priv->line_count++;

    trim_old_lines(panel);
    if (stick_to_bottom) {
        queue_scroll_to_end(panel);
    }
}

static void clear_chat(ChatPanel *panel, const char *channel)
{
    ChatPanelPrivate *priv = panel->priv;
    gtk_text_buffer_set_text(priv->buffer, "", -1);
    priv->line_count = 0;
    priv->follow_tail = TRUE;

    if (channel != NULL) {
        g_autofree char *line = g_strdup_printf("Verbinde mit #%s ...", channel);
        append_status_line(panel, line);
    }
}

static void on_chat_line(const TwitchChatLine *line, gpointer user_data)
{
    ChatPanel *panel = user_data;
    ChatPanelPrivate *priv = panel->priv;

    if (!priv->closing) {
        if (line->kind == TWITCH_CHAT_LINE_MESSAGE) {
            append_message(panel, line);
        } else {
            g_debug("%s", line->message);
            append_status_line(panel, line->message);
        }
    }
}

ChatPanel *chat_panel_new(int width)
{
    ChatPanel *panel = g_new0(ChatPanel, 1);
    ChatPanelPrivate *priv = g_new0(ChatPanelPrivate, 1);
    panel->priv = priv;

    panel->widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(panel->widget, "chat-panel");
    gtk_widget_set_size_request(panel->widget, width, -1);
    gtk_widget_set_vexpand(panel->widget, TRUE);

    priv->scroller = gtk_scrolled_window_new();
    gtk_widget_add_css_class(priv->scroller, "chat-scroll");
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(priv->scroller), GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(priv->scroller, TRUE);
    gtk_widget_set_vexpand(priv->scroller, TRUE);
    gtk_box_append(GTK_BOX(panel->widget), priv->scroller);

    GtkEventController *scroll_controller = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_VERTICAL);
    g_signal_connect(scroll_controller, "scroll", G_CALLBACK(on_chat_scroll), panel);
    gtk_widget_add_controller(priv->scroller, scroll_controller);

    priv->view = gtk_text_view_new();
    gtk_widget_add_css_class(priv->view, "chat-view");
    gtk_widget_set_focusable(priv->view, FALSE);
    gtk_text_view_set_editable(GTK_TEXT_VIEW(priv->view), FALSE);
    gtk_text_view_set_cursor_visible(GTK_TEXT_VIEW(priv->view), FALSE);
    gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(priv->view), GTK_WRAP_WORD_CHAR);
    gtk_text_view_set_left_margin(GTK_TEXT_VIEW(priv->view), 10);
    gtk_text_view_set_right_margin(GTK_TEXT_VIEW(priv->view), 10);
    gtk_text_view_set_top_margin(GTK_TEXT_VIEW(priv->view), 8);
    gtk_text_view_set_bottom_margin(GTK_TEXT_VIEW(priv->view), 8);
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(priv->scroller), priv->view);

    priv->buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(priv->view));
    priv->username_tags = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, NULL);
    priv->assets = chat_assets_new();
    priv->follow_tail = TRUE;
    priv->reply_tag = gtk_text_buffer_create_tag(
        priv->buffer,
        "reply",
        "foreground", "#adadb8",
        "scale", 0.90,
        NULL
    );
    gtk_text_buffer_set_text(priv->buffer, "No chat connected", -1);

    GtkAdjustment *adjustment = gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(priv->scroller));
    if (adjustment != NULL) {
        g_signal_connect(adjustment, "changed", G_CALLBACK(on_chat_adjustment_changed), panel);
    }

    panel->client = twitch_chat_client_new(on_chat_line, panel);

    return panel;
}

void chat_panel_start(ChatPanel *panel, const char *channel)
{
    if (panel == NULL || channel == NULL || channel[0] == '\0') {
        return;
    }

    clear_chat(panel, channel);
    twitch_chat_client_start(panel->client, channel);
}

void chat_panel_free(ChatPanel *panel)
{
    if (panel == NULL) {
        return;
    }

    if (panel->priv != NULL) {
        panel->priv->closing = TRUE;
    }

    twitch_chat_client_free(panel->client);
    panel->client = NULL;

    if (panel->priv != NULL) {
        remove_source_if_active(&panel->priv->scroll_source);

        remove_source_if_active(&panel->priv->scroll_state_source);

        g_clear_pointer(&panel->priv->username_tags, g_hash_table_destroy);
        g_clear_pointer(&panel->priv->assets, chat_assets_free);
    }

    g_clear_pointer(&panel->priv, g_free);
    g_free(panel);
}
