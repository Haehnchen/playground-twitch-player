#define G_LOG_DOMAIN "twitch-player-settings-window"

#include "settings_window.h"
#include "player_icons.h"

typedef struct {
    GtkWidget *window;
    GtkWidget *channels_box;
    GtkWidget *empty_label;
    GtkWidget *status_label;
    AppSettings *settings;
    SettingsWindowSavedCallback saved_callback;
    gpointer user_data;
} SettingsWindow;

typedef struct {
    SettingsWindow *view;
    GtkWidget *row;
    GtkWidget *channel_entry;
} ChannelRow;

static void update_empty_state(SettingsWindow *view)
{
    gboolean has_rows = gtk_widget_get_first_child(view->channels_box) != NULL;
    gtk_widget_set_visible(view->empty_label, !has_rows);
}

static gboolean clear_channel_focus_after_remove(gpointer user_data)
{
    GtkWidget *window = user_data;
    SettingsWindow *view = g_object_get_data(G_OBJECT(window), "settings-window");

    if (view == NULL) {
        return G_SOURCE_REMOVE;
    }

    gtk_window_set_focus(GTK_WINDOW(view->window), NULL);
    for (GtkWidget *child = gtk_widget_get_first_child(view->channels_box);
         child != NULL;
         child = gtk_widget_get_next_sibling(child)) {
        ChannelRow *row = g_object_get_data(G_OBJECT(child), "channel-row");
        if (row != NULL) {
            gtk_editable_select_region(GTK_EDITABLE(row->channel_entry), 0, 0);
        }
    }

    return G_SOURCE_REMOVE;
}

static void on_remove_channel_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    ChannelRow *row = user_data;
    SettingsWindow *view = row->view;
    GtkWidget *row_widget = row->row;

    gtk_box_remove(GTK_BOX(view->channels_box), row_widget);
    update_empty_state(view);
    /* GTK may move focus to the next entry after removal; clear it after layout settles. */
    g_idle_add_full(
        G_PRIORITY_DEFAULT_IDLE,
        clear_channel_focus_after_remove,
        g_object_ref(view->window),
        g_object_unref
    );
}

static gboolean is_valid_channel_name(const char *channel)
{
    if (channel == NULL || channel[0] == '\0') {
        return TRUE;
    }

    for (const char *p = channel; *p != '\0'; p++) {
        if ((*p >= 'a' && *p <= 'z') || (*p >= '0' && *p <= '9') || *p == '_') {
            continue;
        }

        return FALSE;
    }

    return TRUE;
}

static GtkWidget *create_channel_row(SettingsWindow *view, const char *channel)
{
    GtkWidget *row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    gtk_widget_add_css_class(row, "settings-channel-row");

    ChannelRow *row_data = g_new0(ChannelRow, 1);
    row_data->view = view;
    row_data->row = row;
    g_object_set_data_full(G_OBJECT(row), "channel-row", row_data, g_free);

    row_data->channel_entry = gtk_entry_new();
    gtk_entry_set_placeholder_text(GTK_ENTRY(row_data->channel_entry), "Twitch Channel");
    gtk_editable_set_text(GTK_EDITABLE(row_data->channel_entry), channel != NULL ? channel : "");
    gtk_widget_set_hexpand(row_data->channel_entry, TRUE);
    gtk_box_append(GTK_BOX(row), row_data->channel_entry);

    GtkWidget *remove_button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(remove_button), player_trash_icon_new());
    gtk_button_set_has_frame(GTK_BUTTON(remove_button), FALSE);
    gtk_widget_add_css_class(remove_button, "settings-remove-button");
    gtk_widget_set_tooltip_text(remove_button, "Remove");
    gtk_box_append(GTK_BOX(row), remove_button);
    g_signal_connect(remove_button, "clicked", G_CALLBACK(on_remove_channel_clicked), row_data);

    return row;
}

static void add_channel_row(SettingsWindow *view, const char *channel)
{
    gtk_box_append(GTK_BOX(view->channels_box), create_channel_row(view, channel));
    update_empty_state(view);
}

static void on_add_channel_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    add_channel_row(user_data, "");
}

static void on_save_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    SettingsWindow *view = user_data;

    gtk_label_set_text(GTK_LABEL(view->status_label), "");
    app_settings_clear_channels(view->settings);

    for (GtkWidget *child = gtk_widget_get_first_child(view->channels_box);
         child != NULL;
         child = gtk_widget_get_next_sibling(child)) {
        ChannelRow *row = g_object_get_data(G_OBJECT(child), "channel-row");
        if (row == NULL) {
            continue;
        }

        const char *channel = gtk_editable_get_text(GTK_EDITABLE(row->channel_entry));
        g_autofree char *trimmed_channel = g_strdup(channel);
        g_strstrip(trimmed_channel);

        if (trimmed_channel[0] == '\0') {
            continue;
        }

        if (!is_valid_channel_name(trimmed_channel)) {
            gtk_label_set_text(GTK_LABEL(view->status_label), "Invalid channel name. Use a-z, 0-9 and _ only.");
            return;
        }

        app_settings_add_channel(view->settings, NULL, trimmed_channel, NULL);
    }

    g_autoptr(GError) error = NULL;
    if (!app_settings_save(view->settings, &error)) {
        gtk_label_set_text(GTK_LABEL(view->status_label), error->message);
        return;
    }

    if (view->saved_callback != NULL) {
        view->saved_callback(view->settings, view->user_data);
    }

    gtk_window_close(GTK_WINDOW(view->window));
}

static GtkWidget *create_sidebar(void)
{
    GtkWidget *sidebar = gtk_list_box_new();
    gtk_widget_add_css_class(sidebar, "settings-sidebar");
    gtk_widget_set_size_request(sidebar, 170, -1);
    gtk_list_box_set_selection_mode(GTK_LIST_BOX(sidebar), GTK_SELECTION_SINGLE);

    GtkWidget *row = gtk_list_box_row_new();
    GtkWidget *label = gtk_label_new("Channels");
    gtk_widget_add_css_class(label, "settings-sidebar-label");
    gtk_label_set_xalign(GTK_LABEL(label), 0.0);
    gtk_list_box_row_set_child(GTK_LIST_BOX_ROW(row), label);
    gtk_list_box_append(GTK_LIST_BOX(sidebar), row);
    gtk_list_box_select_row(GTK_LIST_BOX(sidebar), GTK_LIST_BOX_ROW(row));

    return sidebar;
}

static GtkWidget *create_channels_page(SettingsWindow *view)
{
    GtkWidget *page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 12);
    gtk_widget_add_css_class(page, "settings-page");
    gtk_widget_set_hexpand(page, TRUE);
    gtk_widget_set_vexpand(page, TRUE);

    GtkWidget *header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    GtkWidget *title = gtk_label_new("Channels");
    gtk_widget_add_css_class(title, "settings-page-title");
    gtk_label_set_xalign(GTK_LABEL(title), 0.0);
    gtk_widget_set_hexpand(title, TRUE);
    gtk_box_append(GTK_BOX(header), title);

    GtkWidget *add_button = gtk_button_new_with_label("Add");
    gtk_widget_add_css_class(add_button, "settings-primary-button");
    gtk_box_append(GTK_BOX(header), add_button);
    g_signal_connect(add_button, "clicked", G_CALLBACK(on_add_channel_clicked), view);
    gtk_box_append(GTK_BOX(page), header);

    view->empty_label = gtk_label_new("No channels saved yet.");
    gtk_widget_add_css_class(view->empty_label, "settings-empty-label");
    gtk_widget_set_halign(view->empty_label, GTK_ALIGN_CENTER);
    gtk_label_set_xalign(GTK_LABEL(view->empty_label), 0.5);
    gtk_box_append(GTK_BOX(page), view->empty_label);

    GtkWidget *scrolled = gtk_scrolled_window_new();
    gtk_widget_set_vexpand(scrolled, TRUE);
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(scrolled), GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);

    view->channels_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_widget_add_css_class(view->channels_box, "settings-channels-box");
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(scrolled), view->channels_box);
    gtk_box_append(GTK_BOX(page), scrolled);

    GtkWidget *footer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    view->status_label = gtk_label_new("");
    gtk_widget_add_css_class(view->status_label, "settings-status-label");
    gtk_label_set_xalign(GTK_LABEL(view->status_label), 0.0);
    gtk_widget_set_hexpand(view->status_label, TRUE);
    gtk_box_append(GTK_BOX(footer), view->status_label);

    GtkWidget *save_button = gtk_button_new_with_label("Save");
    gtk_widget_add_css_class(save_button, "settings-primary-button");
    gtk_box_append(GTK_BOX(footer), save_button);
    g_signal_connect(save_button, "clicked", G_CALLBACK(on_save_clicked), view);
    gtk_box_append(GTK_BOX(page), footer);

    return page;
}

void settings_window_show(
    GtkWindow *parent,
    AppSettings *settings,
    SettingsWindowSavedCallback saved_callback,
    gpointer user_data
)
{
    SettingsWindow *view = g_new0(SettingsWindow, 1);
    view->settings = settings;
    view->saved_callback = saved_callback;
    view->user_data = user_data;

    view->window = gtk_window_new();
    gtk_window_set_title(GTK_WINDOW(view->window), "Settings");
    gtk_window_set_default_size(GTK_WINDOW(view->window), 760, 480);
    gtk_window_set_modal(GTK_WINDOW(view->window), TRUE);
    gtk_window_set_transient_for(GTK_WINDOW(view->window), parent);
    g_object_set_data_full(G_OBJECT(view->window), "settings-window", view, g_free);

    GtkWidget *root = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_add_css_class(root, "settings-window");
    gtk_window_set_child(GTK_WINDOW(view->window), root);

    gtk_box_append(GTK_BOX(root), create_sidebar());
    gtk_box_append(GTK_BOX(root), create_channels_page(view));

    for (guint i = 0; i < app_settings_get_channel_count(settings); i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(settings, i);
        add_channel_row(view, channel->channel);
    }
    update_empty_state(view);

    gtk_window_present(GTK_WINDOW(view->window));
}
