#include "player_overlay_controls.h"

GtkWidget *player_overlay_button_new(GtkWidget *icon, const char *tooltip)
{
    GtkWidget *button = gtk_button_new();
    gtk_button_set_child(GTK_BUTTON(button), icon);
    gtk_button_set_has_frame(GTK_BUTTON(button), FALSE);
    gtk_widget_add_css_class(button, "overlay-icon-button");
    gtk_widget_set_tooltip_text(button, tooltip);
    return button;
}
