#define G_LOG_DOMAIN "chat-panel"

#include "chat_panel.h"

#define MAX_CHAT_LINES 200

struct ChatPanelPrivate {
    GtkWidget *view;
    GtkTextBuffer *buffer;
    GtkTextTag *username_tag;
    guint line_count;
    gboolean closing;
};

static gboolean has_username_prefix(const char *line, const char **colon)
{
    const char *separator = strchr(line, ':');

    if (separator == NULL || separator == line) {
        return FALSE;
    }

    for (const char *p = line; p < separator; p++) {
        if (g_ascii_isspace(*p)) {
            return FALSE;
        }
    }

    *colon = separator;
    return TRUE;
}

static void append_line(ChatPanel *panel, const char *line)
{
    ChatPanelPrivate *priv = panel->priv;
    GtkTextIter end;

    if (priv->closing) {
        return;
    }

    gtk_text_buffer_get_end_iter(priv->buffer, &end);

    const char *colon = NULL;
    if (priv->username_tag != NULL && has_username_prefix(line, &colon)) {
        gtk_text_buffer_insert_with_tags(priv->buffer, &end, line, colon - line, priv->username_tag, NULL);
        gtk_text_buffer_insert(priv->buffer, &end, colon, -1);
    } else {
        gtk_text_buffer_insert(priv->buffer, &end, line, -1);
    }

    gtk_text_buffer_insert(priv->buffer, &end, "\n", -1);
    priv->line_count++;

    while (priv->line_count > MAX_CHAT_LINES) {
        GtkTextIter start;
        GtkTextIter delete_end;

        gtk_text_buffer_get_start_iter(priv->buffer, &start);
        delete_end = start;
        gtk_text_iter_forward_line(&delete_end);
        gtk_text_buffer_delete(priv->buffer, &start, &delete_end);
        priv->line_count--;
    }

    GtkTextMark *insert = gtk_text_buffer_get_insert(priv->buffer);
    gtk_text_view_scroll_mark_onscreen(GTK_TEXT_VIEW(priv->view), insert);
}

static void clear_chat(ChatPanel *panel, const char *channel)
{
    ChatPanelPrivate *priv = panel->priv;
    gtk_text_buffer_set_text(priv->buffer, "", -1);
    priv->line_count = 0;

    if (channel != NULL) {
        g_autofree char *line = g_strdup_printf("Verbinde mit #%s ...", channel);
        append_line(panel, line);
    }
}

static void on_chat_line(const char *line, gpointer user_data)
{
    ChatPanel *panel = user_data;
    ChatPanelPrivate *priv = panel->priv;

    if (!priv->closing) {
        g_debug("%s", line);
        append_line(panel, line);
    }
}

ChatPanel *chat_panel_new(int width)
{
    ChatPanel *panel = g_new0(ChatPanel, 1);
    ChatPanelPrivate *priv = g_new0(ChatPanelPrivate, 1);
    panel->priv = priv;

    panel->widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_set_size_request(panel->widget, width, -1);
    gtk_widget_set_vexpand(panel->widget, TRUE);

    GtkWidget *scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(scroller), GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, TRUE);
    gtk_widget_set_vexpand(scroller, TRUE);
    gtk_box_append(GTK_BOX(panel->widget), scroller);

    priv->view = gtk_text_view_new();
    gtk_text_view_set_editable(GTK_TEXT_VIEW(priv->view), FALSE);
    gtk_text_view_set_cursor_visible(GTK_TEXT_VIEW(priv->view), FALSE);
    gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(priv->view), GTK_WRAP_WORD_CHAR);
    gtk_text_view_set_left_margin(GTK_TEXT_VIEW(priv->view), 6);
    gtk_text_view_set_right_margin(GTK_TEXT_VIEW(priv->view), 6);
    gtk_text_view_set_top_margin(GTK_TEXT_VIEW(priv->view), 4);
    gtk_text_view_set_bottom_margin(GTK_TEXT_VIEW(priv->view), 4);
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(scroller), priv->view);

    priv->buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(priv->view));
    priv->username_tag = gtk_text_buffer_create_tag(priv->buffer, "username", "weight", PANGO_WEIGHT_BOLD, NULL);
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

    g_clear_pointer(&panel->priv, g_free);
    g_free(panel);
}
