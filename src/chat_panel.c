#define G_LOG_DOMAIN "chat-panel"

#include "chat_panel.h"

#define MAX_CHAT_LINES 200

struct ChatPanelPrivate {
    GtkWidget *view;
    GtkTextBuffer *buffer;
    GHashTable *username_tags;
    guint line_count;
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

static void scroll_to_end(ChatPanel *panel)
{
    ChatPanelPrivate *priv = panel->priv;
    GtkTextMark *insert = gtk_text_buffer_get_insert(priv->buffer);

    gtk_text_view_scroll_mark_onscreen(GTK_TEXT_VIEW(priv->view), insert);
}

static void append_status_line(ChatPanel *panel, const char *line)
{
    ChatPanelPrivate *priv = panel->priv;
    GtkTextIter end;

    if (priv->closing) {
        return;
    }

    gtk_text_buffer_get_end_iter(priv->buffer, &end);
    gtk_text_buffer_insert(priv->buffer, &end, line, -1);
    gtk_text_buffer_insert(priv->buffer, &end, "\n", -1);
    priv->line_count++;

    trim_old_lines(panel);
    scroll_to_end(panel);
}

static void append_message(ChatPanel *panel, const TwitchChatLine *line)
{
    ChatPanelPrivate *priv = panel->priv;
    GtkTextIter end;

    if (priv->closing) {
        return;
    }

    gtk_text_buffer_get_end_iter(priv->buffer, &end);

    GtkTextTag *username_tag = get_username_tag(panel, line->display_name, line->color);
    gtk_text_buffer_insert_with_tags(priv->buffer, &end, line->display_name, -1, username_tag, NULL);
    gtk_text_buffer_insert(priv->buffer, &end, ": ", -1);
    gtk_text_buffer_insert(priv->buffer, &end, line->message, -1);
    gtk_text_buffer_insert(priv->buffer, &end, "\n", -1);
    priv->line_count++;

    trim_old_lines(panel);
    scroll_to_end(panel);
}

static void clear_chat(ChatPanel *panel, const char *channel)
{
    ChatPanelPrivate *priv = panel->priv;
    gtk_text_buffer_set_text(priv->buffer, "", -1);
    priv->line_count = 0;

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
            g_debug("%s: %s", line->display_name, line->message);
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

    GtkWidget *scroller = gtk_scrolled_window_new();
    gtk_widget_add_css_class(scroller, "chat-scroll");
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(scroller), GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, TRUE);
    gtk_widget_set_vexpand(scroller, TRUE);
    gtk_box_append(GTK_BOX(panel->widget), scroller);

    priv->view = gtk_text_view_new();
    gtk_widget_add_css_class(priv->view, "chat-view");
    gtk_text_view_set_editable(GTK_TEXT_VIEW(priv->view), FALSE);
    gtk_text_view_set_cursor_visible(GTK_TEXT_VIEW(priv->view), FALSE);
    gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(priv->view), GTK_WRAP_WORD_CHAR);
    gtk_text_view_set_left_margin(GTK_TEXT_VIEW(priv->view), 10);
    gtk_text_view_set_right_margin(GTK_TEXT_VIEW(priv->view), 10);
    gtk_text_view_set_top_margin(GTK_TEXT_VIEW(priv->view), 8);
    gtk_text_view_set_bottom_margin(GTK_TEXT_VIEW(priv->view), 8);
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(scroller), priv->view);

    priv->buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(priv->view));
    priv->username_tags = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, NULL);
    gtk_text_buffer_set_text(priv->buffer, "Kein Chat verbunden", -1);

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

    while (g_main_context_iteration(NULL, FALSE)) {
    }

    if (panel->priv != NULL) {
        g_clear_pointer(&panel->priv->username_tags, g_hash_table_destroy);
    }

    g_clear_pointer(&panel->priv, g_free);
    g_free(panel);
}
