#include "player_volume.h"

#include <math.h>

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
