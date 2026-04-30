#pragma once

#include <glib.h>
#include <mpv/client.h>

typedef struct _PlayerSession PlayerSession;

PlayerSession *player_session_new(void);
void player_session_free(PlayerSession *session);

gboolean player_session_is_ready(PlayerSession *session);
gboolean player_session_is_playing(PlayerSession *session);
mpv_handle *player_session_get_mpv(PlayerSession *session);

const char *player_session_get_label(PlayerSession *session);
const char *player_session_get_channel(PlayerSession *session);
const char *player_session_get_url(PlayerSession *session);
char *player_session_dup_url(PlayerSession *session);

double player_session_get_volume(PlayerSession *session);
void player_session_set_volume(PlayerSession *session, double volume);
gboolean player_session_get_muted(PlayerSession *session);
void player_session_set_muted(PlayerSession *session, gboolean muted);
void player_session_toggle_muted(PlayerSession *session);
void player_session_set_hwdec_enabled(PlayerSession *session, gboolean enabled);
void player_session_set_wakeup_callback(PlayerSession *session, void (*callback)(void *), void *data);
void player_session_reenable_video(PlayerSession *session);
void player_session_load_stream(PlayerSession *session, const char *url, const char *label, const char *channel);
void player_session_stop(PlayerSession *session);
