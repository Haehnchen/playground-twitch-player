#pragma once

#include <gio/gio.h>
#include <glib.h>

#include "twitch_stream_info.h"

typedef struct {
    GCancellable *cancel;
    GPtrArray *qualities;
    char *selected_url;
    char *selected_label;
    gint64 fetched_at;
    guint generation;
    gboolean fetch_in_progress;
} PlayerStreamQualityState;

void player_stream_quality_state_clear(PlayerStreamQualityState *state);
void player_stream_quality_state_reset_selection(PlayerStreamQualityState *state);
gboolean player_stream_quality_state_cache_is_valid(PlayerStreamQualityState *state, guint max_age_seconds);
void player_stream_quality_state_select(PlayerStreamQualityState *state, const TwitchStreamQuality *quality);
void player_stream_quality_state_select_auto(PlayerStreamQualityState *state);
void player_stream_quality_state_cancel_fetch(PlayerStreamQualityState *state);
guint player_stream_quality_state_begin_fetch(PlayerStreamQualityState *state);
void player_stream_quality_state_finish_fetch(PlayerStreamQualityState *state, GPtrArray *qualities);
void player_stream_quality_state_mark_fetched(PlayerStreamQualityState *state);
