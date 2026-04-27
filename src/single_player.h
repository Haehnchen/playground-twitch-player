#pragma once

#include <gtk/gtk.h>

#include "settings.h"

typedef struct _SinglePlayer SinglePlayer;

typedef void (*SinglePlayerFullscreenCallback)(gpointer user_data);

SinglePlayer *single_player_new(
    GtkWindow *window,
    AppSettings *settings,
    const char *startup_target,
    gboolean auto_start,
    SinglePlayerFullscreenCallback fullscreen_callback,
    gpointer fullscreen_user_data
);
GtkWidget *single_player_get_widget(SinglePlayer *player);
void single_player_set_fullscreen(SinglePlayer *player, gboolean fullscreen);
void single_player_show_overlay(SinglePlayer *player);
gboolean single_player_handle_key(
    SinglePlayer *player,
    guint keyval,
    GdkModifierType modifiers
);
void single_player_set_settings(SinglePlayer *player, AppSettings *settings);
void single_player_free(SinglePlayer *player);
