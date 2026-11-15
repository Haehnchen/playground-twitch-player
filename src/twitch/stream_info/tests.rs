use super::ffi::{
    GError, GPtrArray, TwitchStreamQuality, g_free, g_ptr_array_new_with_free_func,
    g_ptr_array_unref,
};
use super::playlist::parse_stream_qualities_playlist;
use super::requests::{build_live_channels_request_body, build_stream_title_request_body};
use super::responses::parse_current_stream_response;
use super::responses::parse_live_channels_response;
use super::responses::parse_playback_info;
use super::responses::{parse_followed_channels_page, parse_helix_user_id_response};
use super::utils::{
    format_live_duration_from_span, format_viewer_count,
    twitch_stream_info_format_current_stream_metadata,
    twitch_stream_info_format_current_stream_title, twitch_stream_info_format_live_duration,
};
use super::utils::{twitch_current_stream_free, twitch_followed_channel_free};
use super::{TwitchCurrentStream, TwitchFollowedChannel, TwitchStreamPreview};
use serde_json::{Value, json};
use std::ffi::{CStr, CString, c_char, c_longlong, c_uint, c_void};
use std::ptr;

const TWITCH_GQL_QUERY: &str =
    "query($login:String!){user(login:$login){stream{title viewersCount createdAt game{name}}}}";

const TWITCH_GQL_LIVE_CHANNELS_QUERY: &str = "query($logins:[String!]!){users(logins:$logins){login displayName profileImageURL(width:70) stream{title viewersCount createdAt previewImageURL(width:320,height:180) game{name}}}}";

const G_TIME_SPAN_MINUTE: c_longlong = 60_000_000;

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn str_from_ptr<'a>(value: *const c_char) -> &'a str {
    unsafe {
        assert!(!value.is_null());
        CStr::from_ptr(value).to_str().unwrap()
    }
}

unsafe fn assert_cstr_eq(actual: *const c_char, expected: &str) {
    unsafe {
        assert_eq!(str_from_ptr(actual), expected);
    }
}

unsafe fn ptr_array_index<T>(array: *mut GPtrArray, index: usize) -> *mut T {
    unsafe { *(*array).pdata.add(index) as *mut T }
}

unsafe extern "C" fn free_followed_channel(data: *mut c_void) {
    unsafe {
        twitch_followed_channel_free(data as *mut TwitchFollowedChannel);
    }
}

#[test]
fn test_build_stream_title_request_body() {
    unsafe {
        let body = build_stream_title_request_body(cstring("papaplatte").as_ptr());
        assert_eq!(
            serde_json::from_str::<Value>(str_from_ptr(body)).unwrap(),
            json!({"query": TWITCH_GQL_QUERY, "variables": {"login": "papaplatte"}}),
        );
        g_free(body as *mut c_void);
    }
}

#[test]
fn test_build_live_channels_request_body() {
    unsafe {
        let channels = [cstring("papaplatte"), cstring("rocketbeans")];
        let ptrs = [channels[0].as_ptr(), channels[1].as_ptr()];
        let body = build_live_channels_request_body(ptrs.as_ptr(), ptrs.len() as c_uint);

        assert_eq!(
            serde_json::from_str::<Value>(str_from_ptr(body)).unwrap(),
            json!({"query": TWITCH_GQL_LIVE_CHANNELS_QUERY, "variables": {"logins": ["papaplatte", "rocketbeans"]}}),
        );
        g_free(body as *mut c_void);
    }
}

#[test]
fn test_build_live_channels_request_body_skips_empty_channels() {
    unsafe {
        let channels = [cstring("papaplatte"), cstring(""), cstring("rocketbeans")];
        let ptrs = [
            channels[0].as_ptr(),
            channels[1].as_ptr(),
            ptr::null(),
            channels[2].as_ptr(),
        ];
        let body = build_live_channels_request_body(ptrs.as_ptr(), ptrs.len() as c_uint);

        assert_eq!(
            serde_json::from_str::<Value>(str_from_ptr(body)).unwrap(),
            json!({"query": TWITCH_GQL_LIVE_CHANNELS_QUERY, "variables": {"logins": ["papaplatte", "rocketbeans"]}}),
        );
        g_free(body as *mut c_void);
    }
}

#[test]
fn test_parse_current_stream_response_returns_title_viewers_and_category() {
    unsafe {
        let json = r#"{"data":{"user":{"stream":{"title":"Live now","viewersCount":1234,"createdAt":"2026-04-30T10:00:00Z","game":{"name":"Games + Demos"}}}}}"#;
        let stream = parse_current_stream_response((json).as_bytes()).unwrap();

        assert!(!stream.is_null());
        assert_cstr_eq((*stream).title, "Live now");
        assert_cstr_eq((*stream).started_at, "2026-04-30T10:00:00Z");
        assert_cstr_eq((*stream).category_name, "Games + Demos");
        assert_eq!((*stream).viewer_count, 1234);
        twitch_current_stream_free(stream);
    }
}

#[test]
fn test_parse_current_stream_response_handles_missing_optional_fields() {
    unsafe {
        let json = r#"{"data":{"user":{"stream":{"title":null,"viewersCount":null,"createdAt":null,"game":null}}}}"#;
        let stream = parse_current_stream_response((json).as_bytes()).unwrap();

        assert!(!stream.is_null());
        assert_cstr_eq((*stream).title, "");
        assert!((*stream).started_at.is_null());
        assert!((*stream).category_name.is_null());
        assert_eq!((*stream).viewer_count, 0);
        twitch_current_stream_free(stream);
    }
}

#[test]
fn test_parse_stream_qualities_playlist_returns_sorted_variants() {
    unsafe {
        let playlist = cstring(
            "#EXTM3U\n\
         #EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720,FRAME-RATE=60.000\n\
         https://example.test/720p60.m3u8\n\
         #EXT-X-STREAM-INF:BANDWIDTH=6000000,RESOLUTION=1920x1080,FRAME-RATE=60.000\n\
         https://example.test/1080p60.m3u8\n\
         #EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=852x480,FRAME-RATE=30.000\n\
         https://example.test/480p.m3u8\n\
         #EXT-X-STREAM-INF:BANDWIDTH=160000,NAME=\"Audio Only\"\n\
         https://example.test/audio.m3u8\n",
        );
        let mut error: *mut GError = ptr::null_mut();
        let qualities = parse_stream_qualities_playlist(playlist.as_ptr(), &mut error);

        assert!(error.is_null());
        assert!(!qualities.is_null());
        assert_eq!((*qualities).len, 3);

        let low = ptr_array_index::<TwitchStreamQuality>(qualities, 0);
        let mid = ptr_array_index::<TwitchStreamQuality>(qualities, 1);
        let source = ptr_array_index::<TwitchStreamQuality>(qualities, 2);

        assert_cstr_eq((*low).label, "480p");
        assert_cstr_eq((*mid).label, "720p60");
        assert_cstr_eq((*source).label, "1080p60");
        assert_cstr_eq((*source).url, "https://example.test/1080p60.m3u8");
        assert_eq!((*source).width, 1920);
        assert_eq!((*source).height, 1080);
        g_ptr_array_unref(qualities);
    }
}

#[test]
fn test_parse_playback_info_returns_authentication_and_ad_claims() {
    let value = r#"{"user_id":123456789,"subscriber":true,"turbo":false,"show_ads":false,"hide_ads":true,"server_ads":false}"#;
    let info = parse_playback_info(value);

    assert_eq!(info.available, 1);
    assert_eq!(info.authenticated, 1);
    assert_eq!(info.user_id, 123456789);
    assert_eq!(info.subscriber, 1);
    assert_eq!(info.turbo, 0);
    assert_eq!(info.show_ads, 0);
    assert_eq!(info.hide_ads, 1);
    assert_eq!(info.server_ads, 0);
}

#[test]
fn test_parse_playback_info_handles_anonymous_and_invalid_values() {
    let anonymous = r#"{"user_id":null,"subscriber":false,"turbo":false,"show_ads":true,"hide_ads":false,"server_ads":true}"#;
    let anonymous_info = parse_playback_info(anonymous);
    let invalid = "{";
    let invalid_info = parse_playback_info(invalid);

    assert_eq!(anonymous_info.available, 1);
    assert_eq!(anonymous_info.authenticated, 0);
    assert_eq!(anonymous_info.user_id, 0);
    assert_eq!(anonymous_info.show_ads, 1);
    assert_eq!(anonymous_info.server_ads, 1);
    assert_eq!(invalid_info.available, 0);
}

#[test]
fn viewer_counts_use_compact_units() {
    for (count, expected) in [
        (0, "0"),
        (999, "999"),
        (1000, "1.0K"),
        (1234, "1.2K"),
        (1_000_000, "1.0M"),
        (1_234_567, "1.2M"),
    ] {
        assert_eq!(format_viewer_count(count), expected);
    }
}

#[test]
fn durations_clamp_future_times_and_use_hours_and_minutes() {
    for (minutes, expected) in [
        (-5, "0m"),
        (0, "0m"),
        (42, "42m"),
        (60, "1h 0m"),
        (65, "1h 5m"),
    ] {
        assert_eq!(
            format_live_duration_from_span(minutes * G_TIME_SPAN_MINUTE),
            expected
        );
    }
}

#[test]
fn invalid_start_date_has_no_duration() {
    unsafe {
        assert!(twitch_stream_info_format_live_duration(c"not a date".as_ptr()).is_null());
    }
}

#[test]
fn test_format_current_stream_title_and_metadata_separately() {
    unsafe {
        let title_value = cstring("Live now");
        let started_value = cstring("2999-01-01T00:00:00Z");
        let category_value = cstring("Games + Demos");
        let stream = TwitchCurrentStream {
            title: title_value.as_ptr() as *mut c_char,
            started_at: started_value.as_ptr() as *mut c_char,
            category_name: category_value.as_ptr() as *mut c_char,
            viewer_count: 1234,
        };
        let untitled_value = cstring("");
        let untitled_category = cstring("Just Chatting");
        let untitled_stream = TwitchCurrentStream {
            title: untitled_value.as_ptr() as *mut c_char,
            started_at: ptr::null_mut(),
            category_name: untitled_category.as_ptr() as *mut c_char,
            viewer_count: 42,
        };

        let title = twitch_stream_info_format_current_stream_title(&stream);
        let untitled = twitch_stream_info_format_current_stream_title(&untitled_stream);
        let metadata = twitch_stream_info_format_current_stream_metadata(&stream);
        let untitled_metadata = twitch_stream_info_format_current_stream_metadata(&untitled_stream);

        assert_cstr_eq(title, "Live now");
        assert_cstr_eq(untitled, "");
        assert_cstr_eq(metadata, "1.2K \u{2022} 0m \u{2022} Games + Demos");
        assert_cstr_eq(untitled_metadata, "42 \u{2022} Just Chatting");
        g_free(title as *mut c_void);
        g_free(untitled as *mut c_void);
        g_free(metadata as *mut c_void);
        g_free(untitled_metadata as *mut c_void);
    }
}

#[test]
fn test_parse_live_channels_response_returns_only_live_streams() {
    unsafe {
        let json = r#"{"data":{"users":[{"login":"LiveOne","displayName":"Live One","profileImageURL":"https://avatar","stream":{"title":"Now live","viewersCount":1234,"createdAt":"2026-04-30T10:00:00Z","game":{"name":"Just Chatting"},"previewImageURL":"https://preview"}},{"login":"BiggerLive","displayName":"Bigger Live","profileImageURL":"https://avatar-big","stream":{"title":"More live","viewersCount":9999,"createdAt":"2026-04-30T09:00:00Z","game":{"name":"Games + Demos"},"previewImageURL":"https://preview-big"}},{"login":"OfflineOne","displayName":"Offline One","profileImageURL":"https://offline","stream":null}]}}"#;
        let previews = parse_live_channels_response((json).as_bytes()).unwrap();

        assert!(!previews.is_null());
        assert_eq!((*previews).len, 2);

        let preview = ptr_array_index::<TwitchStreamPreview>(previews, 0);
        assert_cstr_eq((*preview).channel, "biggerlive");
        assert_cstr_eq((*preview).display_name, "Bigger Live");
        assert_cstr_eq((*preview).title, "More live");
        assert_cstr_eq((*preview).avatar_url, "https://avatar-big");
        assert_cstr_eq((*preview).preview_url, "https://preview-big");
        assert_cstr_eq((*preview).started_at, "2026-04-30T09:00:00Z");
        assert_cstr_eq((*preview).category_name, "Games + Demos");
        assert_eq!((*preview).viewer_count, 9999);

        let preview = ptr_array_index::<TwitchStreamPreview>(previews, 1);
        assert_cstr_eq((*preview).channel, "liveone");
        assert_eq!((*preview).viewer_count, 1234);
        g_ptr_array_unref(previews);
    }
}

#[test]
fn test_parse_live_channels_response_handles_missing_optional_fields() {
    unsafe {
        let json = r#"{"data":{"users":[{"login":"FallbackLogin","displayName":null,"profileImageURL":null,"stream":{"title":null,"viewersCount":null,"createdAt":null,"game":null,"previewImageURL":null}}]}}"#;
        let previews = parse_live_channels_response((json).as_bytes()).unwrap();

        assert!(!previews.is_null());
        assert_eq!((*previews).len, 1);

        let preview = ptr_array_index::<TwitchStreamPreview>(previews, 0);
        assert_cstr_eq((*preview).channel, "fallbacklogin");
        assert_cstr_eq((*preview).display_name, "FallbackLogin");
        assert_cstr_eq((*preview).title, "");
        assert!((*preview).avatar_url.is_null());
        assert!((*preview).preview_url.is_null());
        assert!((*preview).started_at.is_null());
        assert!((*preview).category_name.is_null());
        assert_eq!((*preview).viewer_count, 0);
        g_ptr_array_unref(previews);
    }
}

#[test]
fn test_parse_live_channels_response_sorts_equal_viewers_by_display_name() {
    unsafe {
        let json = r#"{"data":{"users":[{"login":"zeta","displayName":"Zeta","stream":{"title":"Z","viewersCount":10}},{"login":"alpha","displayName":"Alpha","stream":{"title":"A","viewersCount":10}}]}}"#;
        let previews = parse_live_channels_response((json).as_bytes()).unwrap();

        assert!(!previews.is_null());
        assert_eq!((*previews).len, 2);

        assert_cstr_eq(
            (*ptr_array_index::<TwitchStreamPreview>(previews, 0)).display_name,
            "Alpha",
        );
        assert_cstr_eq(
            (*ptr_array_index::<TwitchStreamPreview>(previews, 1)).display_name,
            "Zeta",
        );
        g_ptr_array_unref(previews);
    }
}

#[test]
fn test_parse_live_channels_response_returns_empty_for_missing_users() {
    unsafe {
        let previews = parse_live_channels_response(r#"{"data":{}}"#.as_bytes()).unwrap();

        assert!(!previews.is_null());
        assert_eq!((*previews).len, 0);
        g_ptr_array_unref(previews);
    }
}

#[test]
fn test_parse_helix_user_id_response_returns_user_id() {
    unsafe {
        let user_id = parse_helix_user_id_response(
            r#"{"data":[{"id":"12345","login":"viewer"}]}"#.as_bytes(),
        )
        .unwrap();

        assert_cstr_eq(user_id, "12345");
        g_free(user_id as *mut c_void);
    }
}

#[test]
fn test_parse_followed_channels_page_returns_channels_and_cursor() {
    unsafe {
        let json = cstring(
            r#"{"data":[{"broadcaster_login":"PapaPlatte","broadcaster_name":"Papaplatte"},{"broadcaster_login":"rocketbeans","broadcaster_name":"Rocket Beans TV"}],"pagination":{"cursor":"next-page"}}"#,
        );
        let channels = g_ptr_array_new_with_free_func(Some(free_followed_channel));
        let cursor = parse_followed_channels_page(json.as_bytes(), channels).unwrap();
        assert_eq!((*channels).len, 2);
        assert_cstr_eq(cursor, "next-page");

        let channel = ptr_array_index::<TwitchFollowedChannel>(channels, 0);
        assert_cstr_eq((*channel).channel, "papaplatte");
        assert_cstr_eq((*channel).display_name, "Papaplatte");

        let channel = ptr_array_index::<TwitchFollowedChannel>(channels, 1);
        assert_cstr_eq((*channel).channel, "rocketbeans");
        assert_cstr_eq((*channel).display_name, "Rocket Beans TV");

        g_free(cursor as *mut c_void);
        g_ptr_array_unref(channels);
    }
}

#[test]
fn test_parse_live_channels_response_reports_invalid_json() {
    assert!(parse_live_channels_response(b"{").is_err());
}

#[test]
fn test_parse_current_stream_response_returns_null_for_offline_stream() {
    let stream =
        parse_current_stream_response(r#"{"data":{"user":{"stream":null}}}"#.as_bytes()).unwrap();

    assert!(stream.is_null());
}

#[test]
fn test_parse_current_stream_response_returns_null_for_missing_user() {
    let stream = parse_current_stream_response(r#"{"data":{"user":null}}"#.as_bytes()).unwrap();

    assert!(stream.is_null());
}

#[test]
fn test_parse_current_stream_response_reports_invalid_json() {
    assert!(parse_current_stream_response(b"{").is_err());
}

#[test]
fn playback_request_escapes_query_and_login() {
    use super::requests::{
        TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY, build_playback_access_token_request_body,
    };
    unsafe {
        let login = cstring("ö\\\"\n");
        let body = build_playback_access_token_request_body(login.as_ptr());
        assert_eq!(
            serde_json::from_str::<Value>(str_from_ptr(body)).unwrap(),
            json!({"query": TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY, "variables": {"login": "ö\\\"\n"}}),
        );
        g_free(body.cast());
    }
}

#[test]
fn playback_token_response_reads_nested_claims_and_requires_signature() {
    use super::responses::parse_playback_access_token_response;
    for signature in [json!("sig"), json!(""), Value::Null] {
        let claim = r#"{"user_id":42,"subscriber":true}"#;
        let response = json!({"data": {"streamPlaybackAccessToken": {"value": claim, "signature": signature}}}).to_string();
        let token = parse_playback_access_token_response(response.as_bytes()).unwrap();
        if signature == "sig" {
            let token = token.unwrap();
            assert_eq!(token.value, claim);
            assert_eq!(token.signature, "sig");
            assert_eq!(token.info.user_id, 42);
            assert_eq!(token.info.authenticated, 1);
            assert_eq!(token.info.subscriber, 1);
        } else {
            assert!(token.is_none());
        }
    }
}

#[test]
fn stream_response_handles_wrong_types_and_bounded_viewer_counts() {
    unsafe {
        for viewers in [
            json!(-1),
            json!(4294967296_u64),
            json!("12"),
            json!(null),
            json!(u32::MAX),
        ] {
            let response = json!({"data": {"user": {"stream": {"title": 7, "game": [], "viewersCount": viewers}}}}).to_string();
            let stream = parse_current_stream_response((&response).as_bytes()).unwrap();
            assert!(!stream.is_null());
            assert_cstr_eq((*stream).title, "");
            assert!((*stream).category_name.is_null());
            assert_eq!(
                (*stream).viewer_count,
                if viewers == json!(u32::MAX) {
                    u32::MAX
                } else {
                    0
                }
            );
            twitch_current_stream_free(stream);
        }
    }
}

#[test]
fn helix_pages_skip_invalid_entries_and_use_login_as_display_name() {
    unsafe {
        let response = br#"{"data":[null,{},false,{"broadcaster_login":7},{"broadcaster_login":"\u0000hidden"},{"broadcaster_login":"MixedCase","broadcaster_name":null}],"pagination":{"cursor":null}}trailing"#;
        let channels = g_ptr_array_new_with_free_func(Some(free_followed_channel));
        let length = response.len() - b"trailing".len();
        let cursor = parse_followed_channels_page(&response[..length], channels).unwrap();
        assert_eq!((*channels).len, 1);
        let channel = ptr_array_index::<TwitchFollowedChannel>(channels, 0);
        assert_cstr_eq((*channel).channel, "mixedcase");
        assert_cstr_eq((*channel).display_name, "MixedCase");
        assert!(cursor.is_null());
        g_ptr_array_unref(channels);
    }
}

#[test]
fn hls_parser_handles_quoted_commas_and_excludes_audio_and_iframe_variants() {
    unsafe {
        let playlist = cstring(
            "#EXTM3U\n#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100,RESOLUTION=320x180,URI=\"iframe.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=1234,RESOLUTION=1280x720,FRAME-RATE=59.94,CODECS=\"avc1.64001f,mp4a.40.2\"\nvideo.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=123,NAME=\"Audio Only\"\naudio.m3u8\n",
        );
        let mut error = ptr::null_mut();
        let qualities = parse_stream_qualities_playlist(playlist.as_ptr(), &mut error);
        assert!(error.is_null());
        assert!(!qualities.is_null());
        assert_eq!((*qualities).len, 1);
        let quality = ptr_array_index::<TwitchStreamQuality>(qualities, 0);
        assert_cstr_eq((*quality).label, "720p60");
        assert_cstr_eq((*quality).url, "video.m3u8");
        g_ptr_array_unref(qualities);
    }
}

#[test]
fn hls_parser_reports_invalid_or_video_free_playlists() {
    unsafe {
        for playlist in [
            c"bad playlist",
            c"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=123,NAME=\"Audio Only\"\naudio.m3u8\n",
        ] {
            let mut error = ptr::null_mut();
            assert!(parse_stream_qualities_playlist(playlist.as_ptr(), &mut error).is_null());
            assert!(!error.is_null());
            gtk4::glib::ffi::g_error_free(error.cast());
        }
    }
}
