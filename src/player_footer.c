#include "player_footer.h"

struct _PlayerFooterStreamInfo {
    GtkWidget *widget;
    GtkWidget *title_label;
    GtkWidget *metadata_label;
};

static GtkWidget *create_stream_info_label(const char *css_class, GtkAlign valign)
{
    GtkWidget *label = gtk_label_new("");
    gtk_widget_add_css_class(label, css_class);
    gtk_widget_set_halign(label, GTK_ALIGN_FILL);
    gtk_widget_set_valign(label, valign);
    gtk_widget_set_hexpand(label, TRUE);
    gtk_label_set_xalign(GTK_LABEL(label), 0.0);
    gtk_label_set_ellipsize(GTK_LABEL(label), PANGO_ELLIPSIZE_END);
    gtk_label_set_single_line_mode(GTK_LABEL(label), TRUE);
    return label;
}

PlayerFooterStreamInfo *player_footer_stream_info_new(void)
{
    PlayerFooterStreamInfo *info = g_new0(PlayerFooterStreamInfo, 1);

    info->widget = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(info->widget, "stream-info-labels");
    gtk_widget_set_halign(info->widget, GTK_ALIGN_FILL);
    gtk_widget_set_valign(info->widget, GTK_ALIGN_CENTER);
    gtk_widget_set_hexpand(info->widget, TRUE);
    g_object_add_weak_pointer(G_OBJECT(info->widget), (gpointer *)&info->widget);

    info->title_label = create_stream_info_label("stream-title-label", GTK_ALIGN_END);
    info->metadata_label = create_stream_info_label("stream-metadata-label", GTK_ALIGN_START);
    g_object_add_weak_pointer(G_OBJECT(info->title_label), (gpointer *)&info->title_label);
    g_object_add_weak_pointer(G_OBJECT(info->metadata_label), (gpointer *)&info->metadata_label);

    gtk_box_append(GTK_BOX(info->widget), info->title_label);
    gtk_box_append(GTK_BOX(info->widget), info->metadata_label);

    return info;
}

GtkWidget *player_footer_stream_info_get_widget(PlayerFooterStreamInfo *info)
{
    return info != NULL ? info->widget : NULL;
}

void player_footer_stream_info_set(PlayerFooterStreamInfo *info, const char *title, const char *metadata)
{
    if (info == NULL || info->title_label == NULL) {
        return;
    }

    gtk_label_set_text(GTK_LABEL(info->title_label), title != NULL ? title : "");
    if (info->metadata_label != NULL) {
        gtk_label_set_text(GTK_LABEL(info->metadata_label), metadata != NULL ? metadata : "");
    }

    g_autofree char *tooltip = NULL;
    if (title != NULL && title[0] != '\0' && metadata != NULL && metadata[0] != '\0') {
        tooltip = g_strdup_printf("%s • %s", title, metadata);
    } else if (title != NULL && title[0] != '\0') {
        tooltip = g_strdup(title);
    } else if (metadata != NULL && metadata[0] != '\0') {
        tooltip = g_strdup(metadata);
    }

    gtk_widget_set_tooltip_text(info->title_label, tooltip);
    if (info->metadata_label != NULL) {
        gtk_widget_set_tooltip_text(info->metadata_label, tooltip);
    }
}

void player_footer_stream_info_clear(PlayerFooterStreamInfo *info)
{
    player_footer_stream_info_set(info, "", "");
}

void player_footer_stream_info_free(PlayerFooterStreamInfo *info)
{
    if (info == NULL) {
        return;
    }

    if (info->widget != NULL) {
        g_object_remove_weak_pointer(G_OBJECT(info->widget), (gpointer *)&info->widget);
    }
    if (info->title_label != NULL) {
        g_object_remove_weak_pointer(G_OBJECT(info->title_label), (gpointer *)&info->title_label);
    }
    if (info->metadata_label != NULL) {
        g_object_remove_weak_pointer(G_OBJECT(info->metadata_label), (gpointer *)&info->metadata_label);
    }

    g_free(info);
}
