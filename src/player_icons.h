#pragma once

#include <gtk/gtk.h>

typedef enum {
    PLAYER_WINDOW_ICON_MINIMIZE,
    PLAYER_WINDOW_ICON_FULLSCREEN,
    PLAYER_WINDOW_ICON_CLOSE,
} PlayerWindowIconKind;

typedef enum {
    PLAYER_LAYOUT_ICON_SINGLE,
    PLAYER_LAYOUT_ICON_GRID,
} PlayerLayoutIconKind;

typedef enum {
    PLAYER_CHAT_ICON_OPEN,
    PLAYER_CHAT_ICON_CLOSE,
} PlayerChatIconKind;

typedef enum {
    PLAYER_VOLUME_ICON_SOUND,
    PLAYER_VOLUME_ICON_MUTED,
} PlayerVolumeIconKind;

typedef enum {
    PLAYER_TILE_FOCUS_ICON_EXPAND,
    PLAYER_TILE_FOCUS_ICON_RESTORE,
} PlayerTileFocusIconKind;

GtkWidget *player_settings_icon_new(void);
GtkWidget *player_info_icon_new(void);
GtkWidget *player_trash_icon_new(void);
GtkWidget *player_plus_icon_new(void);
GtkWidget *player_window_icon_new(PlayerWindowIconKind kind);
GtkWidget *player_layout_icon_new(PlayerLayoutIconKind kind);
GtkWidget *player_chat_icon_new(PlayerChatIconKind kind);
GtkWidget *player_volume_icon_new(PlayerVolumeIconKind kind);
GtkWidget *player_tile_focus_icon_new(PlayerTileFocusIconKind kind);
