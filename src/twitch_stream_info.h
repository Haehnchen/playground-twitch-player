#pragma once

#include <gio/gio.h>

void twitch_stream_info_fetch_title_async(
    const char *channel,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

char *twitch_stream_info_fetch_title_finish(GAsyncResult *result, GError **error);
