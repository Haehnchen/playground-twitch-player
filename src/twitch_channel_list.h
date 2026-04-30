#pragma once

#include <gio/gio.h>
#include <glib.h>

#include "settings.h"

void twitch_channel_list_fetch_async(
    const AppSettings *settings,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

char **twitch_channel_list_fetch_finish(GAsyncResult *result, guint *channel_count_out, GError **error);
