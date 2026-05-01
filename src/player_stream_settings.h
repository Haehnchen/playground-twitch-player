#pragma once

#include <gtk/gtk.h>

GtkWidget *player_stream_settings_label_new(const char *text, const char *css_class);
GtkWidget *player_stream_settings_item_button_new(const char *label, gboolean selected);
GtkWidget *player_stream_settings_info_button_new(void);
GtkWidget *player_stream_settings_popover_new(
    GtkWidget *relative_to,
    GtkWidget **quality_list_box_out,
    GtkWidget **quality_status_label_out,
    GtkWidget **info_button_out
);
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
);
