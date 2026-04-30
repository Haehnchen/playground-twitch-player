#pragma once

#include <gtk/gtk.h>

#include "player_session.h"
#include "settings.h"

#define GRID_PLAYER_MAX_TILES 4

typedef struct _GridAppState GridPlayer;

typedef void (*GridPlayerFullscreenCallback)(gpointer user_data);
typedef void (*GridPlayerSettingsCallback)(gpointer user_data);

GridPlayer *grid_player_new(
    GtkWindow *window,
    AppSettings *settings,
    PlayerSession *primary_session,
    const char * const *targets,
    guint target_count,
    GridPlayerFullscreenCallback fullscreen_callback,
    gpointer fullscreen_user_data,
    GridPlayerSettingsCallback settings_callback,
    gpointer settings_user_data
);
GtkWidget *grid_player_get_widget(GridPlayer *player);
char *grid_player_dup_first_target(GridPlayer *player);
PlayerSession *grid_player_take_first_session(GridPlayer *player);
void grid_player_start(GridPlayer *player);
void grid_player_set_fullscreen(GridPlayer *player, gboolean fullscreen);
void grid_player_set_settings(GridPlayer *player, AppSettings *settings);
void grid_player_free(GridPlayer *player);
