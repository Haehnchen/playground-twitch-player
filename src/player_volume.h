#pragma once

#include <gtk/gtk.h>

#include "player_session.h"

#define PLAYER_VOLUME_MIN 0.0
#define PLAYER_VOLUME_MAX 130.0

void player_volume_sync_session_from_range(PlayerSession *session, GtkRange *range);
gboolean player_volume_apply_scroll(GtkWidget *volume_scale, double dx, double dy);
GtkWidget *player_volume_mute_button_new(PlayerSession *session);
void player_volume_update_mute_button(GtkWidget *mute_button, PlayerSession *session);
void player_volume_set_muted(PlayerSession *session, GtkWidget *mute_button, gboolean muted);
void player_volume_toggle_muted(PlayerSession *session, GtkWidget *mute_button);
