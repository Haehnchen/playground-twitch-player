#define G_LOG_DOMAIN "twitch-player-session"

#include "player_session.h"

#include <locale.h>

#include "player_defaults.h"

struct _PlayerSession {
    mpv_handle *mpv;
    char *label;
    char *channel;
    char *url;
    double volume;
    gboolean muted;
    gboolean playing;
};

static void check_mpv(int status, const char *action)
{
    if (status < 0) {
        g_warning("%s: %s", action, mpv_error_string(status));
    }
}

static gboolean init_mpv(PlayerSession *session)
{
    if (setlocale(LC_NUMERIC, "C") == NULL) {
        g_warning("LC_NUMERIC could not be set to C; libmpv may refuse to start");
    }

    session->mpv = mpv_create();
    if (session->mpv == NULL) {
        g_warning("mpv_create returned NULL");
        return FALSE;
    }

    check_mpv(mpv_set_option_string(session->mpv, "terminal", "no"), "set terminal");
    check_mpv(mpv_set_option_string(session->mpv, "config", "no"), "set config");
    check_mpv(mpv_set_option_string(session->mpv, "vo", "libmpv"), "set vo");
    check_mpv(mpv_set_option_string(session->mpv, "ytdl", "yes"), "set ytdl");
    check_mpv(mpv_set_option_string(session->mpv, "hwdec", PLAYER_DEFAULT_MPV_HWDEC), "set hwdec");
    check_mpv(mpv_set_option_string(session->mpv, "cache", PLAYER_DEFAULT_MPV_CACHE), "set cache");
    check_mpv(mpv_set_option_string(session->mpv, "cache-pause-initial", PLAYER_DEFAULT_MPV_CACHE_PAUSE_INITIAL), "set cache pause initial");
    check_mpv(mpv_set_option_string(session->mpv, "cache-pause-wait", PLAYER_DEFAULT_MPV_CACHE_PAUSE_WAIT), "set cache pause wait");
    check_mpv(mpv_set_option_string(session->mpv, "cache-secs", PLAYER_DEFAULT_MPV_CACHE_SECS), "set cache seconds");
    check_mpv(mpv_set_option_string(session->mpv, "demuxer-max-bytes", PLAYER_DEFAULT_MPV_DEMUXER_MAX_BYTES), "set demuxer max bytes");
    check_mpv(mpv_set_option_string(session->mpv, "demuxer-readahead-secs", PLAYER_DEFAULT_MPV_DEMUXER_READAHEAD_SECS), "set demuxer readahead");
    check_mpv(mpv_set_option_string(session->mpv, "stream-buffer-size", PLAYER_DEFAULT_MPV_STREAM_BUFFER_SIZE), "set stream buffer size");
    check_mpv(mpv_set_option_string(session->mpv, "video-sync", PLAYER_DEFAULT_MPV_VIDEO_SYNC), "set video sync");
    check_mpv(mpv_set_option_string(session->mpv, "video-sync-max-audio-change", PLAYER_DEFAULT_MPV_VIDEO_SYNC_MAX_AUDIO_CHANGE), "set video sync audio change");
    check_mpv(mpv_set_option_string(session->mpv, "interpolation", PLAYER_DEFAULT_MPV_INTERPOLATION), "set interpolation");
    check_mpv(mpv_set_option_string(session->mpv, "volume", "80"), "set volume");

    int status = mpv_initialize(session->mpv);
    if (status < 0) {
        g_warning("mpv init: %s", mpv_error_string(status));
        mpv_terminate_destroy(session->mpv);
        session->mpv = NULL;
        return FALSE;
    }

    return TRUE;
}

PlayerSession *player_session_new(void)
{
    PlayerSession *session = g_new0(PlayerSession, 1);
    session->volume = 80.0;
    init_mpv(session);
    return session;
}

void player_session_free(PlayerSession *session)
{
    if (session == NULL) {
        return;
    }

    if (session->mpv != NULL) {
        mpv_set_wakeup_callback(session->mpv, NULL, NULL);
        mpv_terminate_destroy(session->mpv);
        session->mpv = NULL;
    }

    g_free(session->label);
    g_free(session->channel);
    g_free(session->url);
    g_free(session);
}

gboolean player_session_is_ready(PlayerSession *session)
{
    return session != NULL && session->mpv != NULL;
}

gboolean player_session_is_playing(PlayerSession *session)
{
    return player_session_is_ready(session) && session->playing && session->url != NULL && session->url[0] != '\0';
}

mpv_handle *player_session_get_mpv(PlayerSession *session)
{
    return session != NULL ? session->mpv : NULL;
}

const char *player_session_get_label(PlayerSession *session)
{
    return session != NULL ? session->label : NULL;
}

const char *player_session_get_channel(PlayerSession *session)
{
    return session != NULL ? session->channel : NULL;
}

const char *player_session_get_url(PlayerSession *session)
{
    return session != NULL ? session->url : NULL;
}

char *player_session_dup_url(PlayerSession *session)
{
    const char *url = player_session_get_url(session);
    return url != NULL && url[0] != '\0' ? g_strdup(url) : NULL;
}

double player_session_get_volume(PlayerSession *session)
{
    return session != NULL ? session->volume : 80.0;
}

void player_session_set_volume(PlayerSession *session, double volume)
{
    if (!player_session_is_ready(session)) {
        return;
    }

    session->volume = volume;
    check_mpv(mpv_set_property(session->mpv, "volume", MPV_FORMAT_DOUBLE, &volume), "set volume");
}

gboolean player_session_get_muted(PlayerSession *session)
{
    return session != NULL ? session->muted : FALSE;
}

void player_session_set_muted(PlayerSession *session, gboolean muted)
{
    if (!player_session_is_ready(session)) {
        return;
    }

    session->muted = muted;
    check_mpv(mpv_set_property_string(session->mpv, "mute", muted ? "yes" : "no"), muted ? "mute" : "unmute");
}

void player_session_toggle_muted(PlayerSession *session)
{
    player_session_set_muted(session, !player_session_get_muted(session));
}

void player_session_set_wakeup_callback(PlayerSession *session, void (*callback)(void *), void *data)
{
    if (player_session_is_ready(session)) {
        mpv_set_wakeup_callback(session->mpv, callback, data);
    }
}

void player_session_reenable_video(PlayerSession *session)
{
    if (!player_session_is_playing(session)) {
        return;
    }

    check_mpv(mpv_set_property_string(session->mpv, "vid", "no"), "disable video");
    check_mpv(mpv_set_property_string(session->mpv, "vid", "auto"), "enable video");
}

void player_session_load_stream(PlayerSession *session, const char *url, const char *label, const char *channel)
{
    if (!player_session_is_ready(session) || url == NULL || url[0] == '\0') {
        return;
    }

    g_free(session->label);
    g_free(session->channel);
    g_free(session->url);
    session->label = g_strdup(label);
    session->channel = g_strdup(channel);
    session->url = g_strdup(url);
    session->playing = TRUE;

    const char *cmd[] = {
        "loadfile",
        session->url,
        "replace",
        NULL,
    };
    check_mpv(mpv_command_async(session->mpv, 0, cmd), "loadfile");
}

void player_session_stop(PlayerSession *session)
{
    if (!player_session_is_ready(session)) {
        return;
    }

    const char *cmd[] = {
        "stop",
        NULL,
    };
    check_mpv(mpv_command_async(session->mpv, 0, cmd), "stop");

    g_clear_pointer(&session->label, g_free);
    g_clear_pointer(&session->channel, g_free);
    g_clear_pointer(&session->url, g_free);
    session->playing = FALSE;
}
