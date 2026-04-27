#pragma once

#include <gtk/gtk.h>

#include "settings.h"

typedef void (*SettingsWindowSavedCallback)(AppSettings *settings, gpointer user_data);

/**
 * settings_window_show:
 * @parent: Parent application window.
 * @settings: Mutable settings object shown and saved by the dialog.
 * @saved_callback: Optional callback invoked after a successful save.
 * @user_data: User data passed to @saved_callback.
 *
 * Presents the settings window with a sidebar and the Channels settings page.
 * The window edits @settings in place only when Save succeeds.
 */
void settings_window_show(
    GtkWindow *parent,
    AppSettings *settings,
    SettingsWindowSavedCallback saved_callback,
    gpointer user_data
);
