use super::ffi::{
    G_IO_ERROR_FAILED, GCancellable, GError, GPtrArray, JsonParser, TwitchPlaybackInfo,
    g_ascii_strcasecmp, g_ascii_strdown, g_cancellable_is_cancelled, g_free, g_io_error_quark,
    g_object_unref, g_ptr_array_add, g_ptr_array_new_with_free_func, g_ptr_array_sort,
    g_ptr_array_unref, g_set_error, g_strdup, g_uri_escape_string, json_array_get_element,
    json_array_get_length, json_node_get_array, json_node_get_int, json_node_get_object,
    json_object_get_boolean_member_with_default, json_object_get_member,
    json_object_get_string_member_with_default, json_parser_get_root,
};
use super::requests::get_twitch_helix_request;
use super::utils::{
    json_object_get_string_or_null, json_object_get_uint_or_zero, node_is, parser_new_loaded,
};
use super::utils::{twitch_followed_channel_free_destroy, twitch_stream_preview_free_destroy};
use super::{
    JSON_NODE_ARRAY, JSON_NODE_NULL, JSON_NODE_OBJECT, JSON_NODE_VALUE, TwitchCurrentStream,
    TwitchFollowedChannel, TwitchStreamPreview, twitch_playback_info_empty,
};
use crate::glib_strings::is_nonempty;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;

pub(super) unsafe fn parse_current_stream_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut TwitchCurrentStream {
    unsafe {
        let parser = parser_new_loaded(json, length, error);
        if parser.is_null() {
            return ptr::null_mut();
        }

        let result = parse_current_stream_loaded(parser);
        g_object_unref(parser as *mut c_void);
        result
    }
}

pub(super) unsafe fn parse_current_stream_loaded(
    parser: *mut JsonParser,
) -> *mut TwitchCurrentStream {
    unsafe {
        let root = json_parser_get_root(parser);
        if !node_is(root, JSON_NODE_OBJECT) {
            return ptr::null_mut();
        }

        let root_object = json_node_get_object(root);
        let data_node = json_object_get_member(root_object, c"data".as_ptr());
        if !node_is(data_node, JSON_NODE_OBJECT) {
            return ptr::null_mut();
        }

        let data = json_node_get_object(data_node);
        let user_node = json_object_get_member(data, c"user".as_ptr());
        if user_node.is_null()
            || node_is(user_node, JSON_NODE_NULL)
            || !node_is(user_node, JSON_NODE_OBJECT)
        {
            return ptr::null_mut();
        }

        let user = json_node_get_object(user_node);
        let stream_node = json_object_get_member(user, c"stream".as_ptr());
        if stream_node.is_null()
            || node_is(stream_node, JSON_NODE_NULL)
            || !node_is(stream_node, JSON_NODE_OBJECT)
        {
            return ptr::null_mut();
        }

        let stream = json_node_get_object(stream_node);
        let title = json_object_get_string_or_null(stream, c"title".as_ptr());
        let started_at = json_object_get_string_or_null(stream, c"createdAt".as_ptr());
        let mut category_name: *const c_char = ptr::null();
        let game_node = json_object_get_member(stream, c"game".as_ptr());
        if node_is(game_node, JSON_NODE_OBJECT) {
            category_name =
                json_object_get_string_or_null(json_node_get_object(game_node), c"name".as_ptr());
        }

        Box::into_raw(Box::new(TwitchCurrentStream {
            title: if title.is_null() {
                g_strdup(c"".as_ptr())
            } else {
                g_strdup(title)
            },
            started_at: if started_at.is_null() {
                ptr::null_mut()
            } else {
                g_strdup(started_at)
            },
            category_name: if category_name.is_null() {
                ptr::null_mut()
            } else {
                g_strdup(category_name)
            },
            viewer_count: json_object_get_uint_or_zero(stream, c"viewersCount".as_ptr()),
        }))
    }
}

pub(super) unsafe extern "C" fn compare_stream_previews_by_viewers(
    a: *const c_void,
    b: *const c_void,
) -> c_int {
    unsafe {
        let preview_a = *(a as *const *const TwitchStreamPreview);
        let preview_b = *(b as *const *const TwitchStreamPreview);

        if (*preview_a).viewer_count == (*preview_b).viewer_count {
            return g_ascii_strcasecmp((*preview_a).display_name, (*preview_b).display_name);
        }

        if (*preview_a).viewer_count > (*preview_b).viewer_count {
            -1
        } else {
            1
        }
    }
}

pub(super) unsafe fn parse_live_channels_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    unsafe {
        let parser = parser_new_loaded(json, length, error);
        if parser.is_null() {
            return ptr::null_mut();
        }
        let previews = g_ptr_array_new_with_free_func(Some(twitch_stream_preview_free_destroy));
        parse_live_channels_loaded(parser, previews);
        g_object_unref(parser as *mut c_void);
        previews
    }
}

pub(super) unsafe fn parse_live_channels_loaded(parser: *mut JsonParser, previews: *mut GPtrArray) {
    unsafe {
        let root = json_parser_get_root(parser);
        if !node_is(root, JSON_NODE_OBJECT) {
            return;
        }
        let root_object = json_node_get_object(root);
        let data_node = json_object_get_member(root_object, c"data".as_ptr());
        if !node_is(data_node, JSON_NODE_OBJECT) {
            return;
        }
        let users_node = json_object_get_member(json_node_get_object(data_node), c"users".as_ptr());
        if !node_is(users_node, JSON_NODE_ARRAY) {
            return;
        }

        let users = json_node_get_array(users_node);
        for i in 0..json_array_get_length(users) {
            let user_node = json_array_get_element(users, i);
            if user_node.is_null()
                || node_is(user_node, JSON_NODE_NULL)
                || !node_is(user_node, JSON_NODE_OBJECT)
            {
                continue;
            }
            let user = json_node_get_object(user_node);
            let stream_node = json_object_get_member(user, c"stream".as_ptr());
            if stream_node.is_null()
                || node_is(stream_node, JSON_NODE_NULL)
                || !node_is(stream_node, JSON_NODE_OBJECT)
            {
                continue;
            }

            let login = json_object_get_string_or_null(user, c"login".as_ptr());
            if !is_nonempty(login) {
                continue;
            }

            let stream = json_node_get_object(stream_node);
            let title = json_object_get_string_or_null(stream, c"title".as_ptr());
            let preview_url = json_object_get_string_or_null(stream, c"previewImageURL".as_ptr());
            let started_at = json_object_get_string_or_null(stream, c"createdAt".as_ptr());
            let display_name = json_object_get_string_or_null(user, c"displayName".as_ptr());
            let avatar_url = json_object_get_string_or_null(user, c"profileImageURL".as_ptr());
            let mut category_name: *const c_char = ptr::null();
            let game_node = json_object_get_member(stream, c"game".as_ptr());
            if node_is(game_node, JSON_NODE_OBJECT) {
                category_name = json_object_get_string_or_null(
                    json_node_get_object(game_node),
                    c"name".as_ptr(),
                );
            }

            let preview = Box::new(TwitchStreamPreview {
                channel: g_ascii_strdown(login, -1),
                display_name: if is_nonempty(display_name) {
                    g_strdup(display_name)
                } else {
                    g_strdup(login)
                },
                title: if title.is_null() {
                    g_strdup(c"".as_ptr())
                } else {
                    g_strdup(title)
                },
                avatar_url: if avatar_url.is_null() {
                    ptr::null_mut()
                } else {
                    g_strdup(avatar_url)
                },
                preview_url: if preview_url.is_null() {
                    ptr::null_mut()
                } else {
                    g_strdup(preview_url)
                },
                started_at: if started_at.is_null() {
                    ptr::null_mut()
                } else {
                    g_strdup(started_at)
                },
                category_name: if category_name.is_null() {
                    ptr::null_mut()
                } else {
                    g_strdup(category_name)
                },
                viewer_count: json_object_get_uint_or_zero(stream, c"viewersCount".as_ptr()),
            });
            g_ptr_array_add(previews, Box::into_raw(preview) as *mut c_void);
        }

        g_ptr_array_sort(previews, Some(compare_stream_previews_by_viewers));
    }
}

pub(super) unsafe fn parse_playback_info(value: *const c_char) -> TwitchPlaybackInfo {
    unsafe {
        let mut info = twitch_playback_info_empty();
        if !is_nonempty(value) {
            return info;
        }

        let parser = parser_new_loaded(
            value,
            CStr::from_ptr(value).to_bytes().len(),
            ptr::null_mut(),
        );
        if parser.is_null() {
            return info;
        }

        let root = json_parser_get_root(parser);
        if node_is(root, JSON_NODE_OBJECT) {
            let object = json_node_get_object(root);
            let user_id = json_object_get_member(object, c"user_id".as_ptr());
            let user_id = if node_is(user_id, JSON_NODE_VALUE) {
                json_node_get_int(user_id).max(0) as u64
            } else {
                0
            };
            info.available = 1;
            info.authenticated = (user_id > 0) as c_int;
            info.user_id = user_id;
            info.subscriber =
                json_object_get_boolean_member_with_default(object, c"subscriber".as_ptr(), 0);
            info.turbo = json_object_get_boolean_member_with_default(object, c"turbo".as_ptr(), 0);
            info.show_ads =
                json_object_get_boolean_member_with_default(object, c"show_ads".as_ptr(), 0);
            info.hide_ads =
                json_object_get_boolean_member_with_default(object, c"hide_ads".as_ptr(), 0);
            info.server_ads =
                json_object_get_boolean_member_with_default(object, c"server_ads".as_ptr(), 0);
        }

        g_object_unref(parser as *mut c_void);
        info
    }
}

pub(super) unsafe fn parse_playback_access_token_response(
    json: *const c_char,
    length: usize,
    token_out: *mut *mut c_char,
    signature_out: *mut *mut c_char,
    playback_info_out: *mut TwitchPlaybackInfo,
    error: *mut *mut GError,
) -> c_int {
    unsafe {
        let parser = parser_new_loaded(json, length, error);
        if parser.is_null() {
            return 0;
        }

        let ok =
            parse_playback_access_token_loaded(parser, token_out, signature_out, playback_info_out);
        g_object_unref(parser as *mut c_void);
        ok
    }
}

pub(super) unsafe fn parse_playback_access_token_loaded(
    parser: *mut JsonParser,
    token_out: *mut *mut c_char,
    signature_out: *mut *mut c_char,
    playback_info_out: *mut TwitchPlaybackInfo,
) -> c_int {
    unsafe {
        let root = json_parser_get_root(parser);
        if !node_is(root, JSON_NODE_OBJECT) {
            return 0;
        }
        let data_node = json_object_get_member(json_node_get_object(root), c"data".as_ptr());
        if !node_is(data_node, JSON_NODE_OBJECT) {
            return 0;
        }
        let token_node = json_object_get_member(
            json_node_get_object(data_node),
            c"streamPlaybackAccessToken".as_ptr(),
        );
        if token_node.is_null()
            || node_is(token_node, JSON_NODE_NULL)
            || !node_is(token_node, JSON_NODE_OBJECT)
        {
            return 0;
        }

        let token = json_node_get_object(token_node);
        let value = json_object_get_string_or_null(token, c"value".as_ptr());
        let signature = json_object_get_string_or_null(token, c"signature".as_ptr());
        if !is_nonempty(value) || !is_nonempty(signature) {
            return 0;
        }

        *token_out = g_strdup(value);
        *signature_out = g_strdup(signature);
        if !playback_info_out.is_null() {
            *playback_info_out = parse_playback_info(value);
        }
        1
    }
}

pub(super) const TWITCH_HELIX_USERS_URI: &[u8] = b"https://api.twitch.tv/helix/users\0";

pub(super) const TWITCH_HELIX_PAGE_SIZE: &[u8] = b"100";

pub(super) unsafe fn parse_helix_user_id_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut c_char {
    unsafe {
        let parser = parser_new_loaded(json, length, error);
        if parser.is_null() {
            return ptr::null_mut();
        }
        let root = json_parser_get_root(parser);
        let result = if node_is(root, JSON_NODE_OBJECT) {
            let data_node = json_object_get_member(json_node_get_object(root), c"data".as_ptr());
            if node_is(data_node, JSON_NODE_ARRAY) {
                let data = json_node_get_array(data_node);
                if json_array_get_length(data) > 0 {
                    let user_node = json_array_get_element(data, 0);
                    if node_is(user_node, JSON_NODE_OBJECT) {
                        let id = json_object_get_string_member_with_default(
                            json_node_get_object(user_node),
                            c"id".as_ptr(),
                            ptr::null(),
                        );
                        if is_nonempty(id) {
                            g_strdup(id)
                        } else {
                            ptr::null_mut()
                        }
                    } else {
                        ptr::null_mut()
                    }
                } else {
                    ptr::null_mut()
                }
            } else {
                ptr::null_mut()
            }
        } else {
            ptr::null_mut()
        };
        g_object_unref(parser as *mut c_void);
        result
    }
}

pub(super) unsafe fn parse_followed_channels_page(
    json: *const c_char,
    length: usize,
    channels: *mut GPtrArray,
    cursor_out: *mut *mut c_char,
    error: *mut *mut GError,
) -> c_int {
    unsafe {
        let parser = parser_new_loaded(json, length, error);
        if parser.is_null() {
            return 0;
        }

        let root = json_parser_get_root(parser);
        if node_is(root, JSON_NODE_OBJECT) {
            let root_object = json_node_get_object(root);
            let data_node = json_object_get_member(root_object, c"data".as_ptr());
            if node_is(data_node, JSON_NODE_ARRAY) {
                let data = json_node_get_array(data_node);
                for i in 0..json_array_get_length(data) {
                    let channel_node = json_array_get_element(data, i);
                    if !node_is(channel_node, JSON_NODE_OBJECT) {
                        continue;
                    }
                    let channel_object = json_node_get_object(channel_node);
                    let login = json_object_get_string_member_with_default(
                        channel_object,
                        c"broadcaster_login".as_ptr(),
                        ptr::null(),
                    );
                    if !is_nonempty(login) {
                        continue;
                    }
                    let display_name = json_object_get_string_member_with_default(
                        channel_object,
                        c"broadcaster_name".as_ptr(),
                        login,
                    );
                    let channel = Box::new(TwitchFollowedChannel {
                        channel: g_ascii_strdown(login, -1),
                        display_name: if is_nonempty(display_name) {
                            g_strdup(display_name)
                        } else {
                            g_strdup(login)
                        },
                    });
                    g_ptr_array_add(channels, Box::into_raw(channel) as *mut c_void);
                }
            }

            let pagination_node = json_object_get_member(root_object, c"pagination".as_ptr());
            if node_is(pagination_node, JSON_NODE_OBJECT) {
                let cursor = json_object_get_string_member_with_default(
                    json_node_get_object(pagination_node),
                    c"cursor".as_ptr(),
                    ptr::null(),
                );
                if is_nonempty(cursor) {
                    *cursor_out = g_strdup(cursor);
                }
            }
        }

        g_object_unref(parser as *mut c_void);
        1
    }
}

/// Follow pagination cursors until all followed channels are collected.
///
/// # Safety
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
/// - `cancel`: null or live `GCancellable`.
/// - `error`: null or writable, initially null `GError*` slot.
/// - Result owned; release with matching token/array free function.
pub unsafe fn twitch_stream_info_fetch_followed_channels(
    client_id: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    unsafe {
        let user_response = get_twitch_helix_request(
            TWITCH_HELIX_USERS_URI.as_ptr() as *const c_char,
            client_id,
            oauth_token,
            cancel,
            error,
        );
        if user_response.is_null() {
            return ptr::null_mut();
        }

        let user_id = parse_helix_user_id_response(
            user_response,
            CStr::from_ptr(user_response).to_bytes().len(),
            error,
        );
        g_free(user_response as *mut c_void);
        if user_id.is_null() {
            if error.is_null() || (*error).is_null() {
                g_set_error(
                    error,
                    g_io_error_quark(),
                    G_IO_ERROR_FAILED,
                    c"Twitch user could not be read from the token".as_ptr(),
                );
            }
            return ptr::null_mut();
        }

        let channels = g_ptr_array_new_with_free_func(Some(twitch_followed_channel_free_destroy));
        let escaped_user_id = g_uri_escape_string(user_id, ptr::null(), 1);
        let mut cursor: *mut c_char = ptr::null_mut();

        loop {
            let escaped_cursor = if cursor.is_null() {
                ptr::null_mut()
            } else {
                g_uri_escape_string(cursor, ptr::null(), 1)
            };
            let mut request_uri = Vec::new();
            request_uri
                .extend_from_slice(b"https://api.twitch.tv/helix/channels/followed?user_id=");
            request_uri.extend_from_slice(CStr::from_ptr(escaped_user_id).to_bytes());
            request_uri.extend_from_slice(b"&first=");
            request_uri.extend_from_slice(TWITCH_HELIX_PAGE_SIZE);
            if !escaped_cursor.is_null() {
                request_uri.extend_from_slice(b"&after=");
                request_uri.extend_from_slice(CStr::from_ptr(escaped_cursor).to_bytes());
            }
            request_uri.push(0);
            g_free(escaped_cursor as *mut c_void);

            let page_response = get_twitch_helix_request(
                request_uri.as_ptr() as *const c_char,
                client_id,
                oauth_token,
                cancel,
                error,
            );
            if page_response.is_null() {
                g_free(cursor as *mut c_void);
                g_free(escaped_user_id as *mut c_void);
                g_free(user_id as *mut c_void);
                g_ptr_array_unref(channels);
                return ptr::null_mut();
            }

            let mut next_cursor: *mut c_char = ptr::null_mut();
            let ok = parse_followed_channels_page(
                page_response,
                CStr::from_ptr(page_response).to_bytes().len(),
                channels,
                &mut next_cursor,
                error,
            );
            g_free(page_response as *mut c_void);
            if ok == 0 {
                g_free(cursor as *mut c_void);
                g_free(escaped_user_id as *mut c_void);
                g_free(user_id as *mut c_void);
                g_ptr_array_unref(channels);
                return ptr::null_mut();
            }

            g_free(cursor as *mut c_void);
            cursor = next_cursor;
            if cursor.is_null() || (!cancel.is_null() && g_cancellable_is_cancelled(cancel) != 0) {
                break;
            }
        }

        g_free(cursor as *mut c_void);
        g_free(escaped_user_id as *mut c_void);
        g_free(user_id as *mut c_void);
        channels
    }
}
