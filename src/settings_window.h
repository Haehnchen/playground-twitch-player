#pragma once

#include <gtk/gtk.h>

#include "settings.h"

typedef void (*SettingsWindowSavedCallback)(AppSettings *settings, gpointer user_data);

typedef enum {
    SETTINGS_WINDOW_PAGE_GENERAL,
    SETTINGS_WINDOW_PAGE_CHANNELS,
} SettingsWindowPage;

/**
 * settings_window_show:
 * @parent: Parent application window.
 * @settings: Mutable settings object shown and saved by the dialog.
 * @initial_page: Page selected when the dialog opens.
 * @saved_callback: Optional callback invoked after a successful save.
 * @user_data: User data passed to @saved_callback.
 *
 * Presents the settings window with a sidebar and the Channels settings page.
 * The window edits @settings in place only when Save succeeds.
 */
void settings_window_show(
    GtkWindow *parent,
    AppSettings *settings,
    SettingsWindowPage initial_page,
    SettingsWindowSavedCallback saved_callback,
    gpointer user_data
);
