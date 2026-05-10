use std::ffi::{c_char, c_longlong, c_uint, c_void, CStr, CString};
use std::ptr;
use twitch_player_core::twitch_stream_info;

const TWITCH_GQL_QUERY: &str =
    "query($login:String!){user(login:$login){stream{title viewersCount createdAt game{name}}}}";
const TWITCH_GQL_LIVE_CHANNELS_QUERY: &str = "query($logins:[String!]!){users(logins:$logins){login displayName profileImageURL(width:70) stream{title viewersCount createdAt previewImageURL(width:320,height:180) game{name}}}}";
const G_TIME_SPAN_MINUTE: c_longlong = 60_000_000;

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

unsafe extern "C" {
    fn g_clear_error(error: *mut *mut twitch_stream_info::GError);
    fn g_free(mem: *mut c_void);
    fn g_ptr_array_new_with_free_func(
        element_free_func: Option<GDestroyNotify>,
    ) -> *mut twitch_stream_info::GPtrArray;
    fn g_ptr_array_unref(array: *mut twitch_stream_info::GPtrArray);
}

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn str_from_ptr<'a>(value: *const c_char) -> &'a str {
    assert!(!value.is_null());
    CStr::from_ptr(value).to_str().unwrap()
}

unsafe fn assert_cstr_eq(actual: *const c_char, expected: &str) {
    assert_eq!(str_from_ptr(actual), expected);
}

unsafe fn ptr_array_index<T>(array: *mut twitch_stream_info::GPtrArray, index: usize) -> *mut T {
    *(*array).pdata.add(index) as *mut T
}

unsafe fn parse_current(
    json: &str,
    error: *mut *mut twitch_stream_info::GError,
) -> *mut twitch_stream_info::TwitchCurrentStream {
    let json = cstring(json);
    twitch_stream_info::twitch_stream_info_test_parse_current_stream_response(
        json.as_ptr(),
        json.as_bytes().len(),
        error,
    )
}

unsafe fn parse_live(
    json: &str,
    error: *mut *mut twitch_stream_info::GError,
) -> *mut twitch_stream_info::GPtrArray {
    let json = cstring(json);
    twitch_stream_info::twitch_stream_info_test_parse_live_channels_response(
        json.as_ptr(),
        json.as_bytes().len(),
        error,
    )
}

unsafe fn parse_helix_user_id(
    json: &str,
    error: *mut *mut twitch_stream_info::GError,
) -> *mut c_char {
    let json = cstring(json);
    twitch_stream_info::twitch_stream_info_test_parse_helix_user_id_response(
        json.as_ptr(),
        json.as_bytes().len(),
        error,
    )
}

unsafe extern "C" fn free_followed_channel(data: *mut c_void) {
    twitch_stream_info::twitch_followed_channel_free(
        data as *mut twitch_stream_info::TwitchFollowedChannel,
    );
}

unsafe fn test_build_stream_title_request_body() {
    let body = twitch_stream_info::twitch_stream_info_test_build_stream_title_request_body(
        cstring("papaplatte").as_ptr(),
    );
    assert_cstr_eq(
        body,
        &format!(r#"{{"query":"{TWITCH_GQL_QUERY}","variables":{{"login":"papaplatte"}}}}"#),
    );
    g_free(body as *mut c_void);
}

unsafe fn test_build_live_channels_request_body() {
    let channels = [cstring("papaplatte"), cstring("rocketbeans")];
    let ptrs = [channels[0].as_ptr(), channels[1].as_ptr()];
    let body = twitch_stream_info::twitch_stream_info_test_build_live_channels_request_body(
        ptrs.as_ptr(),
        ptrs.len() as c_uint,
    );

    assert_cstr_eq(
        body,
        &format!(
            r#"{{"query":"{TWITCH_GQL_LIVE_CHANNELS_QUERY}","variables":{{"logins":["papaplatte","rocketbeans"]}}}}"#
        ),
    );
    g_free(body as *mut c_void);
}

unsafe fn test_build_live_channels_request_body_skips_empty_channels() {
    let channels = [cstring("papaplatte"), cstring(""), cstring("rocketbeans")];
    let ptrs = [
        channels[0].as_ptr(),
        channels[1].as_ptr(),
        ptr::null(),
        channels[2].as_ptr(),
    ];
    let body = twitch_stream_info::twitch_stream_info_test_build_live_channels_request_body(
        ptrs.as_ptr(),
        ptrs.len() as c_uint,
    );

    assert_cstr_eq(
        body,
        &format!(
            r#"{{"query":"{TWITCH_GQL_LIVE_CHANNELS_QUERY}","variables":{{"logins":["papaplatte","rocketbeans"]}}}}"#
        ),
    );
    g_free(body as *mut c_void);
}

unsafe fn test_parse_current_stream_response_returns_title_viewers_and_category() {
    let json = r#"{"data":{"user":{"stream":{"title":"Live now","viewersCount":1234,"createdAt":"2026-04-30T10:00:00Z","game":{"name":"Games + Demos"}}}}}"#;
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let stream = parse_current(json, &mut error);

    assert!(error.is_null());
    assert!(!stream.is_null());
    assert_cstr_eq((*stream).title, "Live now");
    assert_cstr_eq((*stream).started_at, "2026-04-30T10:00:00Z");
    assert_cstr_eq((*stream).category_name, "Games + Demos");
    assert_eq!((*stream).viewer_count, 1234);
    twitch_stream_info::twitch_current_stream_free(stream);
}

unsafe fn test_parse_current_stream_response_handles_missing_optional_fields() {
    let json = r#"{"data":{"user":{"stream":{"title":null,"viewersCount":null,"createdAt":null,"game":null}}}}"#;
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let stream = parse_current(json, &mut error);

    assert!(error.is_null());
    assert!(!stream.is_null());
    assert_cstr_eq((*stream).title, "");
    assert!((*stream).started_at.is_null());
    assert!((*stream).category_name.is_null());
    assert_eq!((*stream).viewer_count, 0);
    twitch_stream_info::twitch_current_stream_free(stream);
}

unsafe fn test_parse_stream_qualities_playlist_returns_sorted_variants() {
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
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let qualities = twitch_stream_info::twitch_stream_info_test_parse_stream_qualities_playlist(
        playlist.as_ptr(),
        &mut error,
    );

    assert!(error.is_null());
    assert!(!qualities.is_null());
    assert_eq!((*qualities).len, 3);

    let low = ptr_array_index::<twitch_stream_info::TwitchStreamQuality>(qualities, 0);
    let mid = ptr_array_index::<twitch_stream_info::TwitchStreamQuality>(qualities, 1);
    let source = ptr_array_index::<twitch_stream_info::TwitchStreamQuality>(qualities, 2);

    assert_cstr_eq((*low).label, "480p");
    assert_cstr_eq((*mid).label, "720p60");
    assert_cstr_eq((*source).label, "1080p60");
    assert_cstr_eq((*source).url, "https://example.test/1080p60.m3u8");
    assert_eq!((*source).width, 1920);
    assert_eq!((*source).height, 1080);
    g_ptr_array_unref(qualities);
}

unsafe fn test_format_viewer_count_compacts_large_counts() {
    let small = twitch_stream_info::twitch_stream_info_format_viewer_count(999);
    let thousands = twitch_stream_info::twitch_stream_info_format_viewer_count(1234);
    let millions = twitch_stream_info::twitch_stream_info_format_viewer_count(1_234_567);

    assert_cstr_eq(small, "999");
    assert_cstr_eq(thousands, "1.2K");
    assert_cstr_eq(millions, "1.2M");
    g_free(small as *mut c_void);
    g_free(thousands as *mut c_void);
    g_free(millions as *mut c_void);
}

unsafe fn test_format_live_duration_uses_hours_and_minutes() {
    let minutes = twitch_stream_info::twitch_stream_info_test_format_live_duration_from_span(
        42 * G_TIME_SPAN_MINUTE,
    );
    let hours = twitch_stream_info::twitch_stream_info_test_format_live_duration_from_span(
        65 * G_TIME_SPAN_MINUTE,
    );
    let future = twitch_stream_info::twitch_stream_info_test_format_live_duration_from_span(
        -5 * G_TIME_SPAN_MINUTE,
    );
    let invalid =
        twitch_stream_info::twitch_stream_info_format_live_duration(cstring("not a date").as_ptr());

    assert_cstr_eq(minutes, "42m");
    assert_cstr_eq(hours, "1h 5m");
    assert_cstr_eq(future, "0m");
    assert!(invalid.is_null());
    g_free(minutes as *mut c_void);
    g_free(hours as *mut c_void);
    g_free(future as *mut c_void);
}

unsafe fn test_format_current_stream_title_and_metadata_separately() {
    let title_value = cstring("Live now");
    let started_value = cstring("2999-01-01T00:00:00Z");
    let category_value = cstring("Games + Demos");
    let mut stream = twitch_stream_info::TwitchCurrentStream {
        title: title_value.as_ptr() as *mut c_char,
        started_at: started_value.as_ptr() as *mut c_char,
        category_name: category_value.as_ptr() as *mut c_char,
        viewer_count: 1234,
    };
    let untitled_value = cstring("");
    let untitled_category = cstring("Just Chatting");
    let mut untitled_stream = twitch_stream_info::TwitchCurrentStream {
        title: untitled_value.as_ptr() as *mut c_char,
        started_at: ptr::null_mut(),
        category_name: untitled_category.as_ptr() as *mut c_char,
        viewer_count: 42,
    };

    let title = twitch_stream_info::twitch_stream_info_format_current_stream_title(&mut stream);
    let untitled =
        twitch_stream_info::twitch_stream_info_format_current_stream_title(&mut untitled_stream);
    let metadata =
        twitch_stream_info::twitch_stream_info_format_current_stream_metadata(&mut stream);
    let untitled_metadata =
        twitch_stream_info::twitch_stream_info_format_current_stream_metadata(&mut untitled_stream);

    assert_cstr_eq(title, "Live now");
    assert_cstr_eq(untitled, "");
    assert_cstr_eq(metadata, "1.2K \u{2022} 0m \u{2022} Games + Demos");
    assert_cstr_eq(untitled_metadata, "42 \u{2022} Just Chatting");
    g_free(title as *mut c_void);
    g_free(untitled as *mut c_void);
    g_free(metadata as *mut c_void);
    g_free(untitled_metadata as *mut c_void);
}

unsafe fn test_parse_live_channels_response_returns_only_live_streams() {
    let json = r#"{"data":{"users":[{"login":"LiveOne","displayName":"Live One","profileImageURL":"https://avatar","stream":{"title":"Now live","viewersCount":1234,"createdAt":"2026-04-30T10:00:00Z","game":{"name":"Just Chatting"},"previewImageURL":"https://preview"}},{"login":"BiggerLive","displayName":"Bigger Live","profileImageURL":"https://avatar-big","stream":{"title":"More live","viewersCount":9999,"createdAt":"2026-04-30T09:00:00Z","game":{"name":"Games + Demos"},"previewImageURL":"https://preview-big"}},{"login":"OfflineOne","displayName":"Offline One","profileImageURL":"https://offline","stream":null}]}}"#;
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let previews = parse_live(json, &mut error);

    assert!(error.is_null());
    assert!(!previews.is_null());
    assert_eq!((*previews).len, 2);

    let preview = ptr_array_index::<twitch_stream_info::TwitchStreamPreview>(previews, 0);
    assert_cstr_eq((*preview).channel, "biggerlive");
    assert_cstr_eq((*preview).display_name, "Bigger Live");
    assert_cstr_eq((*preview).title, "More live");
    assert_cstr_eq((*preview).avatar_url, "https://avatar-big");
    assert_cstr_eq((*preview).preview_url, "https://preview-big");
    assert_cstr_eq((*preview).started_at, "2026-04-30T09:00:00Z");
    assert_cstr_eq((*preview).category_name, "Games + Demos");
    assert_eq!((*preview).viewer_count, 9999);

    let preview = ptr_array_index::<twitch_stream_info::TwitchStreamPreview>(previews, 1);
    assert_cstr_eq((*preview).channel, "liveone");
    assert_eq!((*preview).viewer_count, 1234);
    g_ptr_array_unref(previews);
}

unsafe fn test_parse_live_channels_response_handles_missing_optional_fields() {
    let json = r#"{"data":{"users":[{"login":"FallbackLogin","displayName":null,"profileImageURL":null,"stream":{"title":null,"viewersCount":null,"createdAt":null,"game":null,"previewImageURL":null}}]}}"#;
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let previews = parse_live(json, &mut error);

    assert!(error.is_null());
    assert!(!previews.is_null());
    assert_eq!((*previews).len, 1);

    let preview = ptr_array_index::<twitch_stream_info::TwitchStreamPreview>(previews, 0);
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

unsafe fn test_parse_live_channels_response_sorts_equal_viewers_by_display_name() {
    let json = r#"{"data":{"users":[{"login":"zeta","displayName":"Zeta","stream":{"title":"Z","viewersCount":10}},{"login":"alpha","displayName":"Alpha","stream":{"title":"A","viewersCount":10}}]}}"#;
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let previews = parse_live(json, &mut error);

    assert!(error.is_null());
    assert!(!previews.is_null());
    assert_eq!((*previews).len, 2);

    assert_cstr_eq(
        (*ptr_array_index::<twitch_stream_info::TwitchStreamPreview>(previews, 0)).display_name,
        "Alpha",
    );
    assert_cstr_eq(
        (*ptr_array_index::<twitch_stream_info::TwitchStreamPreview>(previews, 1)).display_name,
        "Zeta",
    );
    g_ptr_array_unref(previews);
}

unsafe fn test_parse_live_channels_response_returns_empty_for_missing_users() {
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let previews = parse_live(r#"{"data":{}}"#, &mut error);

    assert!(error.is_null());
    assert!(!previews.is_null());
    assert_eq!((*previews).len, 0);
    g_ptr_array_unref(previews);
}

unsafe fn test_parse_helix_user_id_response_returns_user_id() {
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let user_id = parse_helix_user_id(r#"{"data":[{"id":"12345","login":"viewer"}]}"#, &mut error);

    assert!(error.is_null());
    assert_cstr_eq(user_id, "12345");
    g_free(user_id as *mut c_void);
}

unsafe fn test_parse_followed_channels_page_returns_channels_and_cursor() {
    let json = cstring(
        r#"{"data":[{"broadcaster_login":"PapaPlatte","broadcaster_name":"Papaplatte"},{"broadcaster_login":"rocketbeans","broadcaster_name":"Rocket Beans TV"}],"pagination":{"cursor":"next-page"}}"#,
    );
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let channels = g_ptr_array_new_with_free_func(Some(free_followed_channel));
    let mut cursor: *mut c_char = ptr::null_mut();

    assert_ne!(
        twitch_stream_info::twitch_stream_info_test_parse_followed_channels_page(
            json.as_ptr(),
            json.as_bytes().len(),
            channels,
            &mut cursor,
            &mut error,
        ),
        0
    );
    assert!(error.is_null());
    assert_eq!((*channels).len, 2);
    assert_cstr_eq(cursor, "next-page");

    let channel = ptr_array_index::<twitch_stream_info::TwitchFollowedChannel>(channels, 0);
    assert_cstr_eq((*channel).channel, "papaplatte");
    assert_cstr_eq((*channel).display_name, "Papaplatte");

    let channel = ptr_array_index::<twitch_stream_info::TwitchFollowedChannel>(channels, 1);
    assert_cstr_eq((*channel).channel, "rocketbeans");
    assert_cstr_eq((*channel).display_name, "Rocket Beans TV");

    g_free(cursor as *mut c_void);
    g_ptr_array_unref(channels);
}

unsafe fn test_parse_live_channels_response_reports_invalid_json() {
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let previews = parse_live("{", &mut error);

    assert!(previews.is_null());
    assert!(!error.is_null());
    g_clear_error(&mut error);
}

unsafe fn test_parse_current_stream_response_returns_null_for_offline_stream() {
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let stream = parse_current(r#"{"data":{"user":{"stream":null}}}"#, &mut error);

    assert!(error.is_null());
    assert!(stream.is_null());
}

unsafe fn test_parse_current_stream_response_returns_null_for_missing_user() {
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let stream = parse_current(r#"{"data":{"user":null}}"#, &mut error);

    assert!(error.is_null());
    assert!(stream.is_null());
}

unsafe fn test_parse_current_stream_response_reports_invalid_json() {
    let mut error: *mut twitch_stream_info::GError = ptr::null_mut();
    let stream = parse_current("{", &mut error);

    assert!(stream.is_null());
    assert!(!error.is_null());
    g_clear_error(&mut error);
}

fn main() {
    unsafe {
        test_build_stream_title_request_body();
        test_build_live_channels_request_body();
        test_build_live_channels_request_body_skips_empty_channels();
        test_parse_current_stream_response_returns_title_viewers_and_category();
        test_parse_current_stream_response_handles_missing_optional_fields();
        test_parse_stream_qualities_playlist_returns_sorted_variants();
        test_format_viewer_count_compacts_large_counts();
        test_format_live_duration_uses_hours_and_minutes();
        test_format_current_stream_title_and_metadata_separately();
        test_parse_live_channels_response_returns_only_live_streams();
        test_parse_live_channels_response_handles_missing_optional_fields();
        test_parse_live_channels_response_sorts_equal_viewers_by_display_name();
        test_parse_live_channels_response_returns_empty_for_missing_users();
        test_parse_helix_user_id_response_returns_user_id();
        test_parse_followed_channels_page_returns_channels_and_cursor();
        test_parse_live_channels_response_reports_invalid_json();
        test_parse_current_stream_response_returns_null_for_offline_stream();
        test_parse_current_stream_response_returns_null_for_missing_user();
        test_parse_current_stream_response_reports_invalid_json();
    }
}
