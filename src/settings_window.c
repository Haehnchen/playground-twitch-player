#define G_LOG_DOMAIN "twitch-player-settings-window"

#include "settings_window.h"
#include "player_icons.h"
#include "twitch_auth.h"

typedef struct {
    GtkWidget *window;
    GtkWidget *sidebar;
    GtkWidget *stack;
    GtkWidget *hwdec_check;
    GtkWidget *twitch_auth_button;
    GtkWidget *twitch_auth_status;
    GtkWidget *channels_box;
    GtkWidget *empty_label;
    GtkWidget *status_label;
    AppSettings *settings;
    SettingsWindowSavedCallback saved_callback;
    gpointer user_data;
    GCancellable *auth_cancel;
    gboolean auth_in_progress;
} SettingsWindow;

typedef struct {
    SettingsWindow *view;
    GtkWidget *row;
    GtkWidget *channel_entry;
} ChannelRow;

static void disconnect_twitch(SettingsWindow *view);

static const char *page_name_for_page(SettingsWindowPage page)
{
    return page == SETTINGS_WINDOW_PAGE_CHANNELS ? "channels" : "general";
}

static void settings_window_free(SettingsWindow *view)
{
    if (view == NULL) {
        return;
    }

    if (view->auth_cancel != NULL) {
        g_cancellable_cancel(view->auth_cancel);
        g_clear_object(&view->auth_cancel);
    }
    g_free(view);
}

static gboolean has_twitch_auth_client(void)
{
    return TWITCH_AUTH_CLIENT_ID[0] != '\0';
}

static gboolean on_settings_window_close_request(GtkWindow *window, gpointer user_data)
{
    (void)window;
    SettingsWindow *view = user_data;

    if (view->auth_cancel != NULL) {
        g_cancellable_cancel(view->auth_cancel);
    }

    return FALSE;
}

static gboolean has_twitch_auth(SettingsWindow *view)
{
    const char *token = app_settings_get_twitch_oauth_token(view->settings);
    const char *refresh_token = app_settings_get_twitch_refresh_token(view->settings);
    return token != NULL && token[0] != '\0' && refresh_token != NULL && refresh_token[0] != '\0';
}

static void set_twitch_auth_status(SettingsWindow *view, const char *message)
{
    if (view->twitch_auth_status != NULL) {
        gtk_label_set_text(GTK_LABEL(view->twitch_auth_status), message != NULL ? message : "");
    }
}

static void update_twitch_auth_controls(SettingsWindow *view)
{
    if (view->twitch_auth_button == NULL) {
        return;
    }

    gboolean authenticated = has_twitch_auth(view);
    gtk_button_set_label(
        GTK_BUTTON(view->twitch_auth_button),
        authenticated ? "Disconnect Twitch" : "Connect Twitch"
    );
    gtk_widget_set_sensitive(view->twitch_auth_button, has_twitch_auth_client() && !view->auth_in_progress);
}

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

static void finish_twitch_auth(SettingsWindow *view)
{
    view->auth_in_progress = FALSE;
    g_clear_object(&view->auth_cancel);
    update_twitch_auth_controls(view);
}

static void notify_settings_saved(SettingsWindow *view)
{
    if (view->saved_callback != NULL) {
        view->saved_callback(view->settings, view->user_data);
    }
}

static void on_twitch_token_ready(GObject *source_object, GAsyncResult *result, gpointer user_data)
{
    (void)source_object;
    GtkWidget *window = user_data;
    SettingsWindow *view = g_object_get_data(G_OBJECT(window), "settings-window");
    g_autoptr(GError) error = NULL;
    g_autoptr(TwitchAuthToken) token = twitch_auth_poll_device_token_finish(result, &error);

    if (view == NULL) {
        g_object_unref(window);
        return;
    }

    if (token == NULL) {
        if (error != NULL && !g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            g_autofree char *message = g_strdup_printf("Twitch login failed: %s", error->message);
            set_twitch_auth_status(view, message);
        }
        finish_twitch_auth(view);
        g_object_unref(window);
        return;
    }

    gint64 expires_at = token->expires_in > 0
        ? g_get_real_time() / G_USEC_PER_SEC + token->expires_in
        : 0;
    app_settings_set_twitch_auth_tokens(
        view->settings,
        token->access_token,
        token->refresh_token,
        expires_at
    );

    if (!app_settings_save(view->settings, &error)) {
        g_autofree char *message = g_strdup_printf("Twitch login saved in memory, but saving failed: %s", error->message);
        set_twitch_auth_status(view, message);
    } else {
        set_twitch_auth_status(view, "Twitch connected. Followed channels are enabled.");
        notify_settings_saved(view);
    }

    finish_twitch_auth(view);
    g_object_unref(window);
}

static void on_twitch_device_code_ready(GObject *source_object, GAsyncResult *result, gpointer user_data)
{
    (void)source_object;
    GtkWidget *window = user_data;
    SettingsWindow *view = g_object_get_data(G_OBJECT(window), "settings-window");
    g_autoptr(GError) error = NULL;
    g_autoptr(TwitchAuthDeviceCode) code = twitch_auth_request_device_code_finish(result, &error);

    if (view == NULL) {
        g_object_unref(window);
        return;
    }

    if (code == NULL) {
        if (error != NULL && !g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            g_autofree char *message = g_strdup_printf("Twitch login could not start: %s", error->message);
            set_twitch_auth_status(view, message);
        }
        finish_twitch_auth(view);
        g_object_unref(window);
        return;
    }

    GtkUriLauncher *launcher = gtk_uri_launcher_new(code->verification_uri);
    gtk_uri_launcher_launch(launcher, GTK_WINDOW(view->window), NULL, NULL, NULL);
    g_object_unref(launcher);
    g_autofree char *message = g_strdup_printf("Authorize Twitch in the browser with code %s.", code->user_code);
        set_twitch_auth_status(view, message);

    twitch_auth_poll_device_token_async(
        TWITCH_AUTH_CLIENT_ID,
        code,
        view->auth_cancel,
        on_twitch_token_ready,
        g_object_ref(window)
    );
    g_object_unref(window);
}

static void on_twitch_auth_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    SettingsWindow *view = user_data;

    if (has_twitch_auth(view)) {
        disconnect_twitch(view);
        return;
    }

    if (!has_twitch_auth_client()) {
        set_twitch_auth_status(view, "Twitch login is not configured for this build.");
        return;
    }

    if (view->auth_cancel != NULL) {
        g_cancellable_cancel(view->auth_cancel);
        g_clear_object(&view->auth_cancel);
    }

    view->auth_cancel = g_cancellable_new();
    view->auth_in_progress = TRUE;
    update_twitch_auth_controls(view);
    set_twitch_auth_status(view, "Requesting Twitch login code...");

    twitch_auth_request_device_code_async(
        TWITCH_AUTH_CLIENT_ID,
        view->auth_cancel,
        on_twitch_device_code_ready,
        g_object_ref(view->window)
    );
}

static void disconnect_twitch(SettingsWindow *view)
{
    if (view->auth_cancel != NULL) {
        g_cancellable_cancel(view->auth_cancel);
        g_clear_object(&view->auth_cancel);
    }
    view->auth_in_progress = FALSE;

    app_settings_set_twitch_oauth_token(view->settings, NULL);

    g_autoptr(GError) error = NULL;
    if (!app_settings_save(view->settings, &error)) {
        g_autofree char *message = g_strdup_printf("Twitch disconnected in memory, but saving failed: %s", error->message);
        set_twitch_auth_status(view, message);
    } else {
        set_twitch_auth_status(view, "Twitch disconnected.");
        notify_settings_saved(view);
    }

    update_twitch_auth_controls(view);
}

static void on_save_clicked(GtkButton *button, gpointer user_data)
{
    (void)button;
    SettingsWindow *view = user_data;

    gtk_label_set_text(GTK_LABEL(view->status_label), "");
    GPtrArray *channels = g_ptr_array_new_with_free_func(g_free);

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
            g_ptr_array_unref(channels);
            return;
        }

        g_ptr_array_add(channels, g_steal_pointer(&trimmed_channel));
    }

    app_settings_set_hwdec_enabled(
        view->settings,
        view->hwdec_check == NULL || gtk_check_button_get_active(GTK_CHECK_BUTTON(view->hwdec_check))
    );
    app_settings_clear_channels(view->settings);
    for (guint i = 0; i < channels->len; i++) {
        app_settings_add_channel(view->settings, NULL, g_ptr_array_index(channels, i), NULL);
    }
    g_ptr_array_unref(channels);

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

static GtkWidget *create_sidebar_row(const char *name, const char *title)
{
    GtkWidget *row = gtk_list_box_row_new();
    GtkWidget *label = gtk_label_new(title);
    gtk_widget_add_css_class(label, "settings-sidebar-label");
    gtk_label_set_xalign(GTK_LABEL(label), 0.0);
    gtk_list_box_row_set_child(GTK_LIST_BOX_ROW(row), label);
    g_object_set_data_full(G_OBJECT(row), "settings-page", g_strdup(name), g_free);
    return row;
}

static void on_sidebar_row_selected(GtkListBox *box, GtkListBoxRow *row, gpointer user_data)
{
    (void)box;
    SettingsWindow *view = user_data;

    if (row == NULL || view->stack == NULL) {
        return;
    }

    const char *page = g_object_get_data(G_OBJECT(row), "settings-page");
    if (page != NULL) {
        gtk_stack_set_visible_child_name(GTK_STACK(view->stack), page);
    }
}

static GtkWidget *create_sidebar(SettingsWindow *view, SettingsWindowPage initial_page)
{
    GtkWidget *sidebar = gtk_list_box_new();
    gtk_widget_add_css_class(sidebar, "settings-sidebar");
    gtk_widget_set_size_request(sidebar, 170, -1);
    gtk_list_box_set_selection_mode(GTK_LIST_BOX(sidebar), GTK_SELECTION_SINGLE);

    GtkWidget *general_row = create_sidebar_row("general", "General");
    GtkWidget *channels_row = create_sidebar_row("channels", "Channels");
    gtk_list_box_append(GTK_LIST_BOX(sidebar), general_row);
    gtk_list_box_append(GTK_LIST_BOX(sidebar), channels_row);
    g_signal_connect(sidebar, "row-selected", G_CALLBACK(on_sidebar_row_selected), view);
    gtk_list_box_select_row(
        GTK_LIST_BOX(sidebar),
        GTK_LIST_BOX_ROW(initial_page == SETTINGS_WINDOW_PAGE_CHANNELS ? channels_row : general_row)
    );

    return sidebar;
}

static GtkWidget *create_general_page(SettingsWindow *view)
{
    GtkWidget *page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 8);
    gtk_widget_add_css_class(page, "settings-page");
    gtk_widget_set_hexpand(page, TRUE);
    gtk_widget_set_vexpand(page, TRUE);

    GtkWidget *title = gtk_label_new("General");
    gtk_widget_add_css_class(title, "settings-page-title");
    gtk_label_set_xalign(GTK_LABEL(title), 0.0);
    gtk_box_append(GTK_BOX(page), title);

    view->hwdec_check = gtk_check_button_new_with_label("Hardware decoding");
    gtk_check_button_set_active(GTK_CHECK_BUTTON(view->hwdec_check), app_settings_get_hwdec_enabled(view->settings));
    gtk_widget_add_css_class(view->hwdec_check, "settings-check");
    gtk_box_append(GTK_BOX(page), view->hwdec_check);

    GtkWidget *hint = gtk_label_new("Let mpv use GPU video decoding where supported. Disable this if playback is unstable or the video renders incorrectly.");
    gtk_widget_add_css_class(hint, "settings-hint-label");
    gtk_label_set_xalign(GTK_LABEL(hint), 0.0);
    gtk_label_set_wrap(GTK_LABEL(hint), TRUE);
    gtk_widget_set_halign(hint, GTK_ALIGN_FILL);
    gtk_box_append(GTK_BOX(page), hint);

    GtkWidget *spacer = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_set_vexpand(spacer, TRUE);
    gtk_box_append(GTK_BOX(page), spacer);

    return page;
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
    gtk_box_append(GTK_BOX(page), header);

    GtkWidget *twitch_section_title = gtk_label_new("Twitch account");
    gtk_widget_add_css_class(twitch_section_title, "settings-section-title");
    gtk_label_set_xalign(GTK_LABEL(twitch_section_title), 0.0);
    gtk_box_append(GTK_BOX(page), twitch_section_title);

    GtkWidget *followed_hint = gtk_label_new("Connect Twitch to include your followed channels in the channel switcher. Followings are cached for two minutes and are not saved into this list.");
    gtk_widget_add_css_class(followed_hint, "settings-hint-label");
    gtk_label_set_xalign(GTK_LABEL(followed_hint), 0.0);
    gtk_label_set_wrap(GTK_LABEL(followed_hint), TRUE);
    gtk_widget_set_halign(followed_hint, GTK_ALIGN_FILL);
    gtk_box_append(GTK_BOX(page), followed_hint);

    GtkWidget *auth_box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    view->twitch_auth_button = gtk_button_new_with_label("Connect Twitch");
    gtk_widget_add_css_class(view->twitch_auth_button, "settings-primary-button");
    gtk_box_append(GTK_BOX(auth_box), view->twitch_auth_button);
    g_signal_connect(view->twitch_auth_button, "clicked", G_CALLBACK(on_twitch_auth_clicked), view);
    gtk_box_append(GTK_BOX(page), auth_box);

    view->twitch_auth_status = gtk_label_new("");
    gtk_widget_add_css_class(view->twitch_auth_status, "settings-hint-label");
    gtk_label_set_xalign(GTK_LABEL(view->twitch_auth_status), 0.0);
    gtk_label_set_wrap(GTK_LABEL(view->twitch_auth_status), TRUE);
    gtk_widget_set_halign(view->twitch_auth_status, GTK_ALIGN_FILL);
    gtk_box_append(GTK_BOX(page), view->twitch_auth_status);
    if (!has_twitch_auth_client()) {
        set_twitch_auth_status(view, "Twitch login is not configured.");
    } else if (has_twitch_auth(view)) {
        set_twitch_auth_status(view, "Twitch connected.");
    } else if (app_settings_get_twitch_oauth_token(view->settings) != NULL) {
        set_twitch_auth_status(view, "Reconnect Twitch to enable token refresh.");
    }
    update_twitch_auth_controls(view);

    GtkWidget *custom_header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(custom_header, "settings-section-header");
    GtkWidget *custom_title = gtk_label_new("Custom channels");
    gtk_widget_add_css_class(custom_title, "settings-section-title");
    gtk_label_set_xalign(GTK_LABEL(custom_title), 0.0);
    gtk_widget_set_hexpand(custom_title, TRUE);
    gtk_box_append(GTK_BOX(custom_header), custom_title);

    GtkWidget *add_button = gtk_button_new_with_label("Add");
    gtk_widget_add_css_class(add_button, "settings-primary-button");
    gtk_box_append(GTK_BOX(custom_header), add_button);
    g_signal_connect(add_button, "clicked", G_CALLBACK(on_add_channel_clicked), view);
    gtk_box_append(GTK_BOX(page), custom_header);

    GtkWidget *custom_rule = gtk_separator_new(GTK_ORIENTATION_HORIZONTAL);
    gtk_widget_add_css_class(custom_rule, "settings-section-rule");
    gtk_box_append(GTK_BOX(page), custom_rule);

    view->empty_label = gtk_label_new("No channels saved yet.");
    gtk_widget_add_css_class(view->empty_label, "settings-empty-label");
    gtk_widget_set_halign(view->empty_label, GTK_ALIGN_CENTER);
    gtk_label_set_xalign(GTK_LABEL(view->empty_label), 0.5);
    gtk_box_append(GTK_BOX(page), view->empty_label);

    view->channels_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_widget_add_css_class(view->channels_box, "settings-channels-box");
    gtk_widget_set_vexpand(view->channels_box, FALSE);
    gtk_box_append(GTK_BOX(page), view->channels_box);

    return page;
}

static GtkWidget *create_footer(SettingsWindow *view)
{
    GtkWidget *footer = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_add_css_class(footer, "settings-footer");
    view->status_label = gtk_label_new("");
    gtk_widget_add_css_class(view->status_label, "settings-status-label");
    gtk_label_set_xalign(GTK_LABEL(view->status_label), 0.0);
    gtk_widget_set_hexpand(view->status_label, TRUE);
    gtk_box_append(GTK_BOX(footer), view->status_label);

    GtkWidget *save_button = gtk_button_new_with_label("Save");
    gtk_widget_add_css_class(save_button, "settings-primary-button");
    gtk_box_append(GTK_BOX(footer), save_button);
    g_signal_connect(save_button, "clicked", G_CALLBACK(on_save_clicked), view);

    return footer;
}

void settings_window_show(
    GtkWindow *parent,
    AppSettings *settings,
    SettingsWindowPage initial_page,
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
    g_object_set_data_full(G_OBJECT(view->window), "settings-window", view, (GDestroyNotify)settings_window_free);
    g_signal_connect(view->window, "close-request", G_CALLBACK(on_settings_window_close_request), view);

    GtkWidget *root = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_add_css_class(root, "settings-window");
    gtk_window_set_child(GTK_WINDOW(view->window), root);

    view->stack = gtk_stack_new();
    gtk_widget_set_hexpand(view->stack, TRUE);
    gtk_widget_set_vexpand(view->stack, TRUE);
    gtk_stack_add_named(GTK_STACK(view->stack), create_general_page(view), "general");
    gtk_stack_add_named(GTK_STACK(view->stack), create_channels_page(view), "channels");

    GtkWidget *content = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_set_hexpand(content, TRUE);
    gtk_widget_set_vexpand(content, TRUE);
    GtkWidget *scrolled = gtk_scrolled_window_new();
    gtk_widget_set_hexpand(scrolled, TRUE);
    gtk_widget_set_vexpand(scrolled, TRUE);
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(scrolled), GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(scrolled), view->stack);
    gtk_box_append(GTK_BOX(content), scrolled);
    gtk_box_append(GTK_BOX(content), create_footer(view));

    view->sidebar = create_sidebar(view, initial_page);
    gtk_box_append(GTK_BOX(root), view->sidebar);
    gtk_box_append(GTK_BOX(root), content);

    for (guint i = 0; i < app_settings_get_channel_count(settings); i++) {
        const AppSettingsChannel *channel = app_settings_get_channel(settings, i);
        add_channel_row(view, channel->channel);
    }
    update_empty_state(view);
    gtk_stack_set_visible_child_name(GTK_STACK(view->stack), page_name_for_page(initial_page));

    gtk_window_present(GTK_WINDOW(view->window));
}
