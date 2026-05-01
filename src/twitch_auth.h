#pragma once

#include <gio/gio.h>
#include <glib.h>

#define TWITCH_AUTH_CLIENT_ID "8l1fzyh4jhs1cxhtqs6p4swmxuejh6"

typedef struct {
    char *device_code;
    char *user_code;
    char *verification_uri;
    guint expires_in;
    guint interval;
} TwitchAuthDeviceCode;

typedef struct {
    char *access_token;
    char *refresh_token;
    guint expires_in;
} TwitchAuthToken;

void twitch_auth_device_code_free(TwitchAuthDeviceCode *code);
void twitch_auth_token_free(TwitchAuthToken *token);

G_DEFINE_AUTOPTR_CLEANUP_FUNC(TwitchAuthDeviceCode, twitch_auth_device_code_free)
G_DEFINE_AUTOPTR_CLEANUP_FUNC(TwitchAuthToken, twitch_auth_token_free)

void twitch_auth_request_device_code_async(
    const char *client_id,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

TwitchAuthDeviceCode *twitch_auth_request_device_code_finish(GAsyncResult *result, GError **error);

void twitch_auth_poll_device_token_async(
    const char *client_id,
    const TwitchAuthDeviceCode *code,
    GCancellable *cancel,
    GAsyncReadyCallback callback,
    gpointer user_data
);

TwitchAuthToken *twitch_auth_poll_device_token_finish(GAsyncResult *result, GError **error);

TwitchAuthToken *twitch_auth_refresh_token(
    const char *client_id,
    const char *refresh_token,
    GCancellable *cancel,
    GError **error
);
