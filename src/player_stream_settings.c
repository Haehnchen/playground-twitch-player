#include "player_stream_settings.h"

#include "player_icons.h"
#include "twitch_stream_info.h"

GtkWidget *player_stream_settings_label_new(const char *text, const char *css_class)
{
    GtkWidget *label = gtk_label_new(text);
    gtk_label_set_xalign(GTK_LABEL(label), 0.0);
    gtk_widget_set_halign(label, GTK_ALIGN_START);
    gtk_widget_add_css_class(label, css_class);
    return label;
}

GtkWidget *player_stream_settings_item_button_new(const char *label, gboolean selected)
{
    GtkWidget *button = gtk_button_new();
    GtkWidget *button_label = gtk_label_new(label);

    gtk_label_set_xalign(GTK_LABEL(button_label), 0.0);
    gtk_widget_set_halign(button_label, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(button_label, TRUE);
    gtk_button_set_child(GTK_BUTTON(button), button_label);
    gtk_widget_add_css_class(button, "stream-settings-item");
    if (selected) {
        gtk_widget_add_css_class(button, "stream-settings-item-selected");
    }
    gtk_widget_set_halign(button, GTK_ALIGN_FILL);
    return button;
}

GtkWidget *player_stream_settings_info_button_new(void)
{
    GtkWidget *button = gtk_button_new();
    GtkWidget *content = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 6);
    GtkWidget *label = gtk_label_new("Stream Info");

    gtk_widget_add_css_class(button, "stream-settings-item");
    gtk_widget_set_halign(button, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(button, TRUE);
    gtk_widget_set_halign(content, GTK_ALIGN_FILL);
    gtk_widget_set_hexpand(content, TRUE);
    gtk_label_set_xalign(GTK_LABEL(label), 0.0);
    gtk_widget_set_hexpand(label, TRUE);

    gtk_box_append(GTK_BOX(content), player_info_icon_new());
    gtk_box_append(GTK_BOX(content), label);
    gtk_button_set_child(GTK_BUTTON(button), content);

    return button;
}

GtkWidget *player_stream_settings_popover_new(
    GtkWidget *relative_to,
    GtkWidget **quality_list_box_out,
    GtkWidget **quality_status_label_out,
    GtkWidget **info_button_out
)
{
    GtkWidget *popover = gtk_popover_new();
    gtk_widget_add_css_class(popover, "stream-settings-popover");
    gtk_popover_set_position(GTK_POPOVER(popover), GTK_POS_TOP);
    gtk_popover_set_has_arrow(GTK_POPOVER(popover), FALSE);
    gtk_widget_set_parent(popover, relative_to);

    GtkWidget *settings_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_widget_add_css_class(settings_box, "stream-settings-menu");
    gtk_popover_set_child(GTK_POPOVER(popover), settings_box);

    GtkWidget *quality_header = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_set_halign(quality_header, GTK_ALIGN_FILL);
    gtk_widget_set_valign(quality_header, GTK_ALIGN_CENTER);
    gtk_box_append(GTK_BOX(settings_box), quality_header);

    GtkWidget *quality_title = player_stream_settings_label_new("Quality", "stream-settings-heading");
    gtk_widget_set_valign(quality_title, GTK_ALIGN_CENTER);
    gtk_box_append(GTK_BOX(quality_header), quality_title);

    GtkWidget *quality_status_label = player_stream_settings_label_new("", "stream-settings-status");
    gtk_widget_set_valign(quality_status_label, GTK_ALIGN_CENTER);
    gtk_widget_set_hexpand(quality_status_label, FALSE);
    gtk_box_append(GTK_BOX(quality_header), quality_status_label);

    GtkWidget *quality_list_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 2);
    gtk_widget_set_halign(quality_list_box, GTK_ALIGN_FILL);
    gtk_box_append(GTK_BOX(settings_box), quality_list_box);

    GtkWidget *divider = gtk_separator_new(GTK_ORIENTATION_HORIZONTAL);
    gtk_widget_add_css_class(divider, "stream-settings-divider");
    gtk_box_append(GTK_BOX(settings_box), divider);

    GtkWidget *info_button = player_stream_settings_info_button_new();
    gtk_box_append(GTK_BOX(settings_box), info_button);

    if (quality_list_box_out != NULL) {
        *quality_list_box_out = quality_list_box;
    }
    if (quality_status_label_out != NULL) {
        *quality_status_label_out = quality_status_label;
    }
    if (info_button_out != NULL) {
        *info_button_out = info_button;
    }

    return popover;
}

void player_stream_settings_quality_list_populate(
    GtkWidget *quality_list_box,
    GtkWidget *quality_status_label,
    GPtrArray *qualities,
    const char *selected_quality_url,
    const char *selected_quality_label,
    GCallback quality_clicked_callback,
    gpointer quality_user_data,
    GCallback auto_clicked_callback,
    gpointer auto_user_data
)
{
    if (quality_list_box == NULL || quality_status_label == NULL) {
        return;
    }

    GtkWidget *child = gtk_widget_get_first_child(quality_list_box);
    while (child != NULL) {
        GtkWidget *next = gtk_widget_get_next_sibling(child);
        gtk_box_remove(GTK_BOX(quality_list_box), child);
        child = next;
    }

    if (qualities == NULL || qualities->len == 0) {
        gtk_label_set_text(GTK_LABEL(quality_status_label), "No qualities found");
        return;
    }

    gtk_label_set_text(GTK_LABEL(quality_status_label), "");

    for (guint i = 0; i < qualities->len; i++) {
        TwitchStreamQuality *quality = g_ptr_array_index(qualities, i);
        gboolean selected =
            (selected_quality_url != NULL && g_strcmp0(selected_quality_url, quality->url) == 0) ||
            (selected_quality_label != NULL && g_strcmp0(selected_quality_label, quality->label) == 0);
        g_autofree char *label = selected ? g_strdup_printf("%s (current)", quality->label) : g_strdup(quality->label);
        GtkWidget *button = player_stream_settings_item_button_new(label, selected);
        g_object_set_data(G_OBJECT(button), "stream-quality", quality);
        g_signal_connect(button, "clicked", quality_clicked_callback, quality_user_data);
        gtk_box_append(GTK_BOX(quality_list_box), button);
    }

    gboolean auto_selected = selected_quality_url == NULL;
    GtkWidget *auto_button = player_stream_settings_item_button_new(
        auto_selected ? "Auto (current)" : "Auto",
        auto_selected
    );
    g_signal_connect(auto_button, "clicked", auto_clicked_callback, auto_user_data);
    gtk_box_append(GTK_BOX(quality_list_box), auto_button);
}
