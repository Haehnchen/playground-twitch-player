#include "player_stream_quality.h"

void player_stream_quality_state_clear(PlayerStreamQualityState *state)
{
    if (state == NULL) {
        return;
    }

    player_stream_quality_state_cancel_fetch(state);
    g_clear_pointer(&state->qualities, g_ptr_array_unref);
    player_stream_quality_state_select_auto(state);
    state->fetched_at = 0;
    state->generation++;
}

void player_stream_quality_state_reset_selection(PlayerStreamQualityState *state)
{
    if (state == NULL) {
        return;
    }

    player_stream_quality_state_clear(state);
}

gboolean player_stream_quality_state_cache_is_valid(PlayerStreamQualityState *state, guint max_age_seconds)
{
    return state != NULL &&
        state->qualities != NULL &&
        state->fetched_at > 0 &&
        g_get_monotonic_time() - state->fetched_at < (gint64)max_age_seconds * G_USEC_PER_SEC;
}

void player_stream_quality_state_select(PlayerStreamQualityState *state, const TwitchStreamQuality *quality)
{
    if (state == NULL || quality == NULL) {
        return;
    }

    g_free(state->selected_url);
    g_free(state->selected_label);
    state->selected_url = g_strdup(quality->url);
    state->selected_label = g_strdup(quality->label);
}

void player_stream_quality_state_select_auto(PlayerStreamQualityState *state)
{
    if (state == NULL) {
        return;
    }

    g_clear_pointer(&state->selected_url, g_free);
    g_clear_pointer(&state->selected_label, g_free);
}

void player_stream_quality_state_cancel_fetch(PlayerStreamQualityState *state)
{
    if (state == NULL) {
        return;
    }

    if (state->cancel != NULL) {
        g_cancellable_cancel(state->cancel);
        g_clear_object(&state->cancel);
    }
    state->fetch_in_progress = FALSE;
}

guint player_stream_quality_state_begin_fetch(PlayerStreamQualityState *state)
{
    if (state == NULL) {
        return 0;
    }

    state->cancel = g_cancellable_new();
    state->fetch_in_progress = TRUE;
    return ++state->generation;
}

void player_stream_quality_state_finish_fetch(PlayerStreamQualityState *state, GPtrArray *qualities)
{
    if (state == NULL) {
        if (qualities != NULL) {
            g_ptr_array_unref(qualities);
        }
        return;
    }

    state->fetch_in_progress = FALSE;
    g_clear_object(&state->cancel);
    g_clear_pointer(&state->qualities, g_ptr_array_unref);
    state->qualities = qualities;
}

void player_stream_quality_state_mark_fetched(PlayerStreamQualityState *state)
{
    if (state != NULL) {
        state->fetched_at = g_get_monotonic_time();
    }
}
