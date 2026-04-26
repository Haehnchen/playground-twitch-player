#define G_LOG_DOMAIN "chat-panel"

#include "chat_panel.h"

struct ChatPanelPrivate {
    GtkWidget *view;
    GtkTextBuffer *buffer;
    gboolean closing;
};

static void append_line(ChatPanel *panel, const char *line)
{
    struct ChatPanelPrivate *priv = g_object_get_data(G_OBJECT(panel->widget), "chat-panel-private");
    GtkTextIter end;

    gtk_text_buffer_get_end_iter(priv->buffer, &end);
    gtk_text_buffer_insert(priv->buffer, &end, line, -1);
    gtk_text_buffer_insert(priv->buffer, &end, "\n", -1);

    GtkTextMark *insert = gtk_text_buffer_get_insert(priv->buffer);
    gtk_text_view_scroll_mark_onscreen(GTK_TEXT_VIEW(priv->view), insert);
}

static void clear_chat(ChatPanel *panel, const char *channel)
{
    struct ChatPanelPrivate *priv = g_object_get_data(G_OBJECT(panel->widget), "chat-panel-private");
    gtk_text_buffer_set_text(priv->buffer, "", -1);

    if (channel != NULL) {
        g_autofree char *line = g_strdup_printf("Verbinde mit #%s ...", channel);
        append_line(panel, line);
    }
}

static void on_chat_line(const char *line, gpointer user_data)
{
    ChatPanel *panel = user_data;
    struct ChatPanelPrivate *priv = g_object_get_data(G_OBJECT(panel->widget), "chat-panel-private");

    if (!priv->closing) {
        g_debug("%s", line);
        append_line(panel, line);
    }
}

static void free_private(gpointer data)
{
    struct ChatPanelPrivate *priv = data;
    priv->closing = TRUE;
    g_free(priv);
}

ChatPanel *chat_panel_new(int width)
{
    ChatPanel *panel = g_new0(ChatPanel, 1);
    struct ChatPanelPrivate *priv = g_new0(struct ChatPanelPrivate, 1);

    panel->widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_widget_set_size_request(panel->widget, width, -1);
    gtk_widget_set_vexpand(panel->widget, TRUE);
    g_object_set_data_full(G_OBJECT(panel->widget), "chat-panel-private", priv, free_private);

    GtkWidget *title = gtk_label_new("Chat");
    gtk_widget_set_halign(title, GTK_ALIGN_START);
    gtk_box_append(GTK_BOX(panel->widget), title);

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

    twitch_chat_client_free(panel->client);
    panel->client = NULL;

    if (panel->widget != NULL) {
        struct ChatPanelPrivate *priv = g_object_get_data(G_OBJECT(panel->widget), "chat-panel-private");
        if (priv != NULL) {
            priv->closing = TRUE;
        }
    }

    g_free(panel);
}
