#include "player_volume.h"

#include <math.h>

#include "player_icons.h"
#include "player_overlay_controls.h"

#define PLAYER_VOLUME_SCROLL_STEP 5.0

void player_volume_sync_session_from_range(PlayerSession *session, GtkRange *range)
{
    if (range == NULL) {
        return;
    }

    player_session_set_volume(session, gtk_range_get_value(range));
}

gboolean player_volume_apply_scroll(GtkWidget *volume_scale, double dx, double dy)
{
    if (!GTK_IS_RANGE(volume_scale) || fabs(dy) < fabs(dx) || dy == 0.0) {
        return FALSE;
    }

    GtkRange *range = GTK_RANGE(volume_scale);
    double volume = gtk_range_get_value(range);

    volume = CLAMP(volume - dy * PLAYER_VOLUME_SCROLL_STEP, PLAYER_VOLUME_MIN, PLAYER_VOLUME_MAX);
    gtk_range_set_value(range, volume);

    return TRUE;
}

GtkWidget *player_volume_mute_button_new(PlayerSession *session)
{
    GtkWidget *button = player_overlay_button_new(
        player_volume_icon_new(
            player_session_get_muted(session) ? PLAYER_VOLUME_ICON_MUTED : PLAYER_VOLUME_ICON_SOUND
        ),
        NULL
    );
    gtk_widget_add_css_class(button, "volume-mute-button");
    return button;
}

void player_volume_update_mute_button(GtkWidget *mute_button, PlayerSession *session)
{
    if (!GTK_IS_BUTTON(mute_button)) {
        return;
    }

    gboolean muted = player_session_get_muted(session);
    gtk_button_set_child(
        GTK_BUTTON(mute_button),
        player_volume_icon_new(muted ? PLAYER_VOLUME_ICON_MUTED : PLAYER_VOLUME_ICON_SOUND)
    );
}

void player_volume_set_muted(PlayerSession *session, GtkWidget *mute_button, gboolean muted)
{
    player_session_set_muted(session, muted);
    player_volume_update_mute_button(mute_button, session);
}

void player_volume_toggle_muted(PlayerSession *session, GtkWidget *mute_button)
{
    player_session_toggle_muted(session);
    player_volume_update_mute_button(mute_button, session);
}
