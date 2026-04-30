#pragma once

#include <gtk/gtk.h>

#include "settings.h"

typedef struct _ChannelSwitcherOverlay ChannelSwitcherOverlay;

typedef void (*ChannelSwitcherActivateCallback)(
    const AppSettingsChannel *channel,
    gpointer user_data
);

ChannelSwitcherOverlay *channel_switcher_overlay_new(
    GtkOverlay *overlay,
    AppSettings *settings,
    ChannelSwitcherActivateCallback activate_callback,
    gpointer user_data
);

void channel_switcher_overlay_set_settings(ChannelSwitcherOverlay *switcher, AppSettings *settings);
void channel_switcher_overlay_show_at(ChannelSwitcherOverlay *switcher, double x, double y);
void channel_switcher_overlay_hide(ChannelSwitcherOverlay *switcher);
gboolean channel_switcher_overlay_is_visible(ChannelSwitcherOverlay *switcher);
void channel_switcher_overlay_free(ChannelSwitcherOverlay *switcher);
