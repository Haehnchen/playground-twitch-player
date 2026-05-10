use std::ffi::{c_char, c_double, c_int, c_uint, c_void, CStr};
use std::ptr;

const JSON_NODE_OBJECT: c_int = 0;
const JSON_NODE_ARRAY: c_int = 1;
const JSON_NODE_VALUE: c_int = 2;
const JSON_NODE_NULL: c_int = 3;
const G_IO_ERROR_FAILED: c_int = 0;
const G_IO_ERROR_NOT_FOUND: c_int = 1;
const TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED: c_int = 0;
const TWITCH_GQL_URI: &[u8] = b"https://gql.twitch.tv/gql\0";
const TWITCH_GQL_CLIENT_ID: &[u8] = b"kimne78kx3ncx6brgo4mv6wki5h1ko\0";
const TWITCH_GQL_QUERY: &str =
    "query($login:String!){user(login:$login){stream{title viewersCount createdAt game{name}}}}";
const TWITCH_GQL_LIVE_CHANNELS_QUERY: &str = "query($logins:[String!]!){users(logins:$logins){login displayName profileImageURL(width:70) stream{title viewersCount createdAt previewImageURL(width:320,height:180) game{name}}}}";
const TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY: &str = "query($login:String!){streamPlaybackAccessToken(channelName:$login,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:\"site\"}){value signature}}";
const TWITCH_GQL_MAX_LOGINS: c_uint = 100;
const TWITCH_HELIX_USERS_URI: &[u8] = b"https://api.twitch.tv/helix/users\0";
const TWITCH_HELIX_PAGE_SIZE: &[u8] = b"100";

pub struct TwitchStreamPreview {
    pub channel: *mut c_char,
    pub display_name: *mut c_char,
    pub title: *mut c_char,
    pub avatar_url: *mut c_char,
    pub preview_url: *mut c_char,
    pub started_at: *mut c_char,
    pub category_name: *mut c_char,
    pub viewer_count: c_uint,
}

pub struct TwitchCurrentStream {
    pub title: *mut c_char,
    pub started_at: *mut c_char,
    pub category_name: *mut c_char,
    pub viewer_count: c_uint,
}

pub struct TwitchStreamQuality {
    pub label: *mut c_char,
    pub url: *mut c_char,
    pub width: c_uint,
    pub height: c_uint,
    pub bandwidth: c_uint,
    pub frame_rate: c_double,
}

pub struct TwitchFollowedChannel {
    pub channel: *mut c_char,
    pub display_name: *mut c_char,
}

pub struct FetchCurrentStreamData {
    channel: *mut c_char,
}

pub struct FetchStreamQualitiesData {
    channel: *mut c_char,
}

pub struct FetchLiveChannelsData {
    channels: *mut *mut c_char,
    channel_count: c_uint,
}

pub struct FetchFollowedChannelsData {
    client_id: *mut c_char,
    oauth_token: *mut c_char,
}

#[repr(C)]
pub struct GAsyncResult {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GBytes {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GCancellable {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GError {
    pub domain: c_uint,
    pub code: c_int,
    pub message: *mut c_char,
}

#[repr(C)]
pub struct GPtrArray {
    pub pdata: *mut *mut c_void,
    pub len: c_uint,
}

#[repr(C)]
pub struct GTask {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonArray {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonNode {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonParser {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupMessage {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupMessageHeaders {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SoupSession {
    _private: [u8; 0],
}

pub type GAsyncReadyCallback = unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void);
type GCompareFunc = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;
type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GTaskThreadFunc =
    unsafe extern "C" fn(*mut GTask, *mut c_void, *mut c_void, *mut GCancellable);

unsafe extern "C" {
    fn g_ascii_strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn g_ascii_strdown(str: *const c_char, len: isize) -> *mut c_char;
    fn g_ascii_strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn g_ascii_strtoull(nptr: *const c_char, endptr: *mut *mut c_char, base: c_uint) -> u64;
    fn g_bytes_get_data(bytes: *mut GBytes, size: *mut usize) -> *const c_void;
    fn g_bytes_new_static(data: *const c_void, size: usize) -> *mut GBytes;
    fn g_bytes_unref(bytes: *mut GBytes);
    fn g_date_time_difference(end: *mut GDateTime, begin: *mut GDateTime) -> i64;
    fn g_date_time_new_from_iso8601(text: *const c_char, default_tz: *mut c_void)
        -> *mut GDateTime;
    fn g_date_time_new_now_utc() -> *mut GDateTime;
    fn g_date_time_unref(datetime: *mut GDateTime);
    fn g_free(mem: *mut c_void);
    fn g_io_error_quark() -> c_uint;
    fn g_malloc0(n_bytes: usize) -> *mut c_void;
    fn g_object_set(object: *mut c_void, first_property_name: *const c_char, ...);
    fn g_object_unref(object: *mut c_void);
    fn g_ptr_array_add(array: *mut GPtrArray, data: *mut c_void);
    fn g_ptr_array_new_with_free_func(element_free_func: Option<GDestroyNotify>) -> *mut GPtrArray;
    fn g_ptr_array_set_free_func(array: *mut GPtrArray, element_free_func: Option<GDestroyNotify>);
    fn g_ptr_array_sort(array: *mut GPtrArray, compare_func: Option<GCompareFunc>);
    fn g_ptr_array_unref(array: *mut GPtrArray);
    fn g_quark_from_static_string(string: *const c_char) -> c_uint;
    fn g_random_int_range(begin: c_int, end: c_int) -> c_int;
    fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_strndup(str: *const c_char, n: usize) -> *mut c_char;
    fn g_task_is_valid(result: *mut c_void, source_object: *mut c_void) -> c_int;
    fn g_task_new(
        source_object: *mut c_void,
        cancellable: *mut GCancellable,
        callback: Option<GAsyncReadyCallback>,
        callback_data: *mut c_void,
    ) -> *mut GTask;
    fn g_task_propagate_pointer(task: *mut GTask, error: *mut *mut GError) -> *mut c_void;
    fn g_task_return_error(task: *mut GTask, error: *mut GError);
    fn g_task_return_pointer(
        task: *mut GTask,
        result: *mut c_void,
        result_destroy: Option<GDestroyNotify>,
    );
    fn g_task_run_in_thread(task: *mut GTask, task_func: Option<GTaskThreadFunc>);
    fn g_task_set_task_data(
        task: *mut GTask,
        task_data: *mut c_void,
        task_data_destroy: Option<GDestroyNotify>,
    );
    fn g_uri_escape_string(
        unescaped: *const c_char,
        reserved_chars_allowed: *const c_char,
        allow_utf8: c_int,
    ) -> *mut c_char;

    fn json_array_get_element(array: *mut JsonArray, index_: c_uint) -> *mut JsonNode;
    fn json_array_get_length(array: *mut JsonArray) -> c_uint;
    fn json_node_get_array(node: *mut JsonNode) -> *mut JsonArray;
    fn json_node_get_int(node: *mut JsonNode) -> i64;
    fn json_node_get_node_type(node: *mut JsonNode) -> c_int;
    fn json_node_get_object(node: *mut JsonNode) -> *mut JsonObject;
    fn json_node_get_string(node: *mut JsonNode) -> *const c_char;
    fn json_object_get_member(object: *mut JsonObject, member_name: *const c_char)
        -> *mut JsonNode;
    fn json_object_get_string_member_with_default(
        object: *mut JsonObject,
        member_name: *const c_char,
        default_value: *const c_char,
    ) -> *const c_char;
    fn json_parser_get_root(parser: *mut JsonParser) -> *mut JsonNode;
    fn json_parser_load_from_data(
        parser: *mut JsonParser,
        data: *const c_char,
        length: isize,
        error: *mut *mut GError,
    ) -> c_int;
    fn json_parser_new() -> *mut JsonParser;

    fn soup_message_get_request_headers(message: *mut SoupMessage) -> *mut SoupMessageHeaders;
    fn soup_message_get_status(message: *mut SoupMessage) -> c_uint;
    fn soup_message_headers_append(
        hdrs: *mut SoupMessageHeaders,
        name: *const c_char,
        value: *const c_char,
    );
    fn soup_message_new(method: *const c_char, uri_string: *const c_char) -> *mut SoupMessage;
    fn soup_message_set_request_body_from_bytes(
        msg: *mut SoupMessage,
        content_type: *const c_char,
        bytes: *mut GBytes,
    );
    fn soup_session_new() -> *mut SoupSession;
    fn soup_session_send_and_read(
        session: *mut SoupSession,
        msg: *mut SoupMessage,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut GBytes;
}

#[repr(C)]
pub struct GDateTime {
    _private: [u8; 0],
}

unsafe fn is_nonempty(value: *const c_char) -> bool {
    !value.is_null() && *value != 0
}

unsafe fn dup_bytes(bytes: &[u8]) -> *mut c_char {
    let mut value = Vec::with_capacity(bytes.len() + 1);
    value.extend_from_slice(bytes);
    value.push(0);
    g_strdup(value.as_ptr() as *const c_char)
}

fn json_escape_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(bytes.len() + 8);
    for &byte in bytes {
        match byte {
            b'"' => escaped.extend_from_slice(br#"\""#),
            b'\\' => escaped.extend_from_slice(br#"\\"#),
            b'\n' => escaped.extend_from_slice(br#"\n"#),
            b'\r' => escaped.extend_from_slice(br#"\r"#),
            b'\t' => escaped.extend_from_slice(br#"\t"#),
            0x00..=0x1f => escaped.extend_from_slice(format!("\\u{byte:04x}").as_bytes()),
            other => escaped.push(other),
        }
    }
    escaped
}

unsafe fn c_bytes(value: *const c_char) -> Vec<u8> {
    if value.is_null() {
        Vec::new()
    } else {
        CStr::from_ptr(value).to_bytes().to_vec()
    }
}

unsafe fn json_string_from_c(value: *const c_char) -> Vec<u8> {
    json_escape_bytes(&c_bytes(value))
}

unsafe fn build_gql_login_body(query: &str, channel: *const c_char) -> *mut c_char {
    let login = json_string_from_c(channel);
    let mut body = Vec::new();
    body.extend_from_slice(br#"{"query":""#);
    body.extend_from_slice(json_escape_bytes(query.as_bytes()).as_slice());
    body.extend_from_slice(br#"","variables":{"login":""#);
    body.extend_from_slice(&login);
    body.extend_from_slice(br#""}}"#);
    dup_bytes(&body)
}

unsafe fn build_stream_title_request_body(channel: *const c_char) -> *mut c_char {
    build_gql_login_body(TWITCH_GQL_QUERY, channel)
}

unsafe fn build_live_channels_request_body(
    channels: *const *const c_char,
    channel_count: c_uint,
) -> *mut c_char {
    let mut body = Vec::new();
    body.extend_from_slice(br#"{"query":""#);
    body.extend_from_slice(json_escape_bytes(TWITCH_GQL_LIVE_CHANNELS_QUERY.as_bytes()).as_slice());
    body.extend_from_slice(br#"","variables":{"logins":["#);
    let mut first = true;

    for i in 0..channel_count {
        let channel = *channels.add(i as usize);
        if !is_nonempty(channel) {
            continue;
        }

        if !first {
            body.push(b',');
        }
        first = false;
        body.push(b'"');
        body.extend_from_slice(&json_string_from_c(channel));
        body.push(b'"');
    }

    body.extend_from_slice(br#"]}}"#);
    dup_bytes(&body)
}

unsafe fn build_playback_access_token_request_body(channel: *const c_char) -> *mut c_char {
    build_gql_login_body(TWITCH_GQL_PLAYBACK_ACCESS_TOKEN_QUERY, channel)
}

unsafe fn node_is(node: *mut JsonNode, kind: c_int) -> bool {
    !node.is_null() && json_node_get_node_type(node) == kind
}

unsafe fn json_object_get_string_or_null(
    object: *mut JsonObject,
    member_name: *const c_char,
) -> *const c_char {
    let node = json_object_get_member(object, member_name);

    if node.is_null() || node_is(node, JSON_NODE_NULL) || !node_is(node, JSON_NODE_VALUE) {
        return ptr::null();
    }

    json_node_get_string(node)
}

unsafe fn json_object_get_uint_or_zero(
    object: *mut JsonObject,
    member_name: *const c_char,
) -> c_uint {
    let node = json_object_get_member(object, member_name);

    if node.is_null() || node_is(node, JSON_NODE_NULL) || !node_is(node, JSON_NODE_VALUE) {
        return 0;
    }

    let value = json_node_get_int(node);
    if value > 0 && value <= c_uint::MAX as i64 {
        value as c_uint
    } else {
        0
    }
}

unsafe fn parser_new_loaded(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut JsonParser {
    let parser = json_parser_new();
    if json_parser_load_from_data(parser, json, length as isize, error) == 0 {
        g_object_unref(parser as *mut c_void);
        return ptr::null_mut();
    }

    parser
}

unsafe fn parse_current_stream_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut TwitchCurrentStream {
    let parser = parser_new_loaded(json, length, error);
    if parser.is_null() {
        return ptr::null_mut();
    }

    let result = parse_current_stream_loaded(parser);
    g_object_unref(parser as *mut c_void);
    result
}

unsafe fn parse_current_stream_loaded(parser: *mut JsonParser) -> *mut TwitchCurrentStream {
    let root = json_parser_get_root(parser);
    if !node_is(root, JSON_NODE_OBJECT) {
        return ptr::null_mut();
    }

    let root_object = json_node_get_object(root);
    let data_node = json_object_get_member(root_object, b"data\0".as_ptr() as *const c_char);
    if !node_is(data_node, JSON_NODE_OBJECT) {
        return ptr::null_mut();
    }

    let data = json_node_get_object(data_node);
    let user_node = json_object_get_member(data, b"user\0".as_ptr() as *const c_char);
    if user_node.is_null()
        || node_is(user_node, JSON_NODE_NULL)
        || !node_is(user_node, JSON_NODE_OBJECT)
    {
        return ptr::null_mut();
    }

    let user = json_node_get_object(user_node);
    let stream_node = json_object_get_member(user, b"stream\0".as_ptr() as *const c_char);
    if stream_node.is_null()
        || node_is(stream_node, JSON_NODE_NULL)
        || !node_is(stream_node, JSON_NODE_OBJECT)
    {
        return ptr::null_mut();
    }

    let stream = json_node_get_object(stream_node);
    let title = json_object_get_string_or_null(stream, b"title\0".as_ptr() as *const c_char);
    let started_at =
        json_object_get_string_or_null(stream, b"createdAt\0".as_ptr() as *const c_char);
    let mut category_name: *const c_char = ptr::null();
    let game_node = json_object_get_member(stream, b"game\0".as_ptr() as *const c_char);
    if node_is(game_node, JSON_NODE_OBJECT) {
        category_name = json_object_get_string_or_null(
            json_node_get_object(game_node),
            b"name\0".as_ptr() as *const c_char,
        );
    }

    Box::into_raw(Box::new(TwitchCurrentStream {
        title: if title.is_null() {
            g_strdup(b"\0".as_ptr() as *const c_char)
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
        viewer_count: json_object_get_uint_or_zero(
            stream,
            b"viewersCount\0".as_ptr() as *const c_char,
        ),
    }))
}

unsafe fn parse_playback_access_token_response(
    json: *const c_char,
    length: usize,
    token_out: *mut *mut c_char,
    signature_out: *mut *mut c_char,
    error: *mut *mut GError,
) -> c_int {
    let parser = parser_new_loaded(json, length, error);
    if parser.is_null() {
        return 0;
    }

    let ok = parse_playback_access_token_loaded(parser, token_out, signature_out);
    g_object_unref(parser as *mut c_void);
    ok
}

unsafe fn parse_playback_access_token_loaded(
    parser: *mut JsonParser,
    token_out: *mut *mut c_char,
    signature_out: *mut *mut c_char,
) -> c_int {
    let root = json_parser_get_root(parser);
    if !node_is(root, JSON_NODE_OBJECT) {
        return 0;
    }
    let data_node = json_object_get_member(
        json_node_get_object(root),
        b"data\0".as_ptr() as *const c_char,
    );
    if !node_is(data_node, JSON_NODE_OBJECT) {
        return 0;
    }
    let token_node = json_object_get_member(
        json_node_get_object(data_node),
        b"streamPlaybackAccessToken\0".as_ptr() as *const c_char,
    );
    if token_node.is_null()
        || node_is(token_node, JSON_NODE_NULL)
        || !node_is(token_node, JSON_NODE_OBJECT)
    {
        return 0;
    }

    let token = json_node_get_object(token_node);
    let value = json_object_get_string_or_null(token, b"value\0".as_ptr() as *const c_char);
    let signature = json_object_get_string_or_null(token, b"signature\0".as_ptr() as *const c_char);
    if !is_nonempty(value) || !is_nonempty(signature) {
        return 0;
    }

    *token_out = g_strdup(value);
    *signature_out = g_strdup(signature);
    1
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

fn get_m3u_attribute_value_bytes(line: &[u8], name: &[u8]) -> Option<Vec<u8>> {
    let mut needle = Vec::with_capacity(name.len() + 1);
    needle.extend_from_slice(name);
    needle.push(b'=');
    let start = line
        .windows(needle.len())
        .position(|window| window == needle.as_slice())?;
    let mut value_start = start + needle.len();

    if line.get(value_start) == Some(&b'"') {
        value_start += 1;
        let end = line[value_start..]
            .iter()
            .position(|byte| *byte == b'"')
            .map(|idx| value_start + idx)
            .unwrap_or(line.len());
        return Some(line[value_start..end].to_vec());
    }

    let end = line[value_start..]
        .iter()
        .position(|byte| *byte == b',')
        .map(|idx| value_start + idx)
        .unwrap_or(line.len());
    Some(line[value_start..end].to_vec())
}

unsafe fn parse_uint_attribute(line: &[u8], name: &[u8]) -> c_uint {
    let Some(value) = get_m3u_attribute_value_bytes(line, name) else {
        return 0;
    };
    if value.is_empty() {
        return 0;
    }

    let value = dup_bytes(&value);
    let parsed = g_ascii_strtoull(value, ptr::null_mut(), 10);
    g_free(value as *mut c_void);
    if parsed <= c_uint::MAX as u64 {
        parsed as c_uint
    } else {
        0
    }
}

unsafe fn parse_double_attribute(line: &[u8], name: &[u8]) -> c_double {
    let Some(value) = get_m3u_attribute_value_bytes(line, name) else {
        return 0.0;
    };
    if value.is_empty() {
        return 0.0;
    }

    let value = dup_bytes(&value);
    let parsed = g_ascii_strtod(value, ptr::null_mut());
    g_free(value as *mut c_void);
    parsed
}

unsafe fn parse_resolution_attribute(line: &[u8], width: *mut c_uint, height: *mut c_uint) {
    let Some(resolution) = get_m3u_attribute_value_bytes(line, b"RESOLUTION") else {
        return;
    };
    let Some(separator) = resolution.iter().position(|byte| *byte == b'x') else {
        return;
    };

    let width_text = dup_bytes(&resolution[..separator]);
    let height_text = dup_bytes(&resolution[separator + 1..]);
    let parsed_width = g_ascii_strtoull(width_text, ptr::null_mut(), 10);
    let parsed_height = g_ascii_strtoull(height_text, ptr::null_mut(), 10);
    g_free(width_text as *mut c_void);
    g_free(height_text as *mut c_void);

    if parsed_width <= c_uint::MAX as u64 && parsed_height <= c_uint::MAX as u64 {
        *width = parsed_width as c_uint;
        *height = parsed_height as c_uint;
    }
}

unsafe fn build_quality_label(
    _width: c_uint,
    height: c_uint,
    frame_rate: c_double,
    bandwidth: c_uint,
) -> *mut c_char {
    let label = if height > 0 {
        if frame_rate >= 50.0 {
            format!("{height}p60")
        } else {
            format!("{height}p")
        }
    } else if bandwidth > 0 {
        format!("{} kbps", std::cmp::max(1, bandwidth / 1000))
    } else {
        "Unknown".to_string()
    };
    dup_bytes(label.as_bytes())
}

unsafe extern "C" fn compare_stream_qualities(a: *const c_void, b: *const c_void) -> c_int {
    let quality_a = *(a as *const *const TwitchStreamQuality);
    let quality_b = *(b as *const *const TwitchStreamQuality);

    if (*quality_a).height != (*quality_b).height {
        return if (*quality_a).height < (*quality_b).height {
            -1
        } else {
            1
        };
    }
    if (*quality_a).frame_rate != (*quality_b).frame_rate {
        return if (*quality_a).frame_rate < (*quality_b).frame_rate {
            -1
        } else {
            1
        };
    }
    if (*quality_a).bandwidth != (*quality_b).bandwidth {
        return if (*quality_a).bandwidth < (*quality_b).bandwidth {
            -1
        } else {
            1
        };
    }

    g_ascii_strcasecmp((*quality_a).label, (*quality_b).label)
}

unsafe fn parse_stream_qualities_playlist(
    playlist: *const c_char,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    let qualities = g_ptr_array_new_with_free_func(Some(twitch_stream_quality_free_destroy));
    let playlist = if playlist.is_null() {
        &[][..]
    } else {
        CStr::from_ptr(playlist).to_bytes()
    };
    let mut pending_stream_info: Option<Vec<u8>> = None;

    for raw_line in playlist.split(|byte| *byte == b'\n') {
        let line = trim_ascii(raw_line);

        if line.starts_with(b"#EXT-X-STREAM-INF:") {
            pending_stream_info = Some(line.to_vec());
            continue;
        }

        if pending_stream_info.is_none() || line.is_empty() || line[0] == b'#' {
            continue;
        }

        let pending = pending_stream_info.take().unwrap();
        let mut quality = Box::new(TwitchStreamQuality {
            label: ptr::null_mut(),
            url: ptr::null_mut(),
            width: 0,
            height: 0,
            bandwidth: parse_uint_attribute(&pending, b"BANDWIDTH"),
            frame_rate: parse_double_attribute(&pending, b"FRAME-RATE"),
        });
        parse_resolution_attribute(&pending, &mut quality.width, &mut quality.height);

        let name = get_m3u_attribute_value_bytes(&pending, b"NAME");
        let lower_name = name.map(|mut value| {
            value.make_ascii_lowercase();
            value
        });
        if quality.height == 0
            || lower_name
                .as_ref()
                .is_some_and(|name| name.windows(5).any(|window| window == b"audio"))
        {
            continue;
        }

        quality.label = build_quality_label(
            quality.width,
            quality.height,
            quality.frame_rate,
            quality.bandwidth,
        );
        quality.url = dup_bytes(line);
        g_ptr_array_add(qualities, Box::into_raw(quality) as *mut c_void);
    }

    g_ptr_array_sort(qualities, Some(compare_stream_qualities));

    if (*qualities).len == 0 {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_NOT_FOUND,
            b"No HLS stream variants found\0".as_ptr() as *const c_char,
        );
        g_ptr_array_unref(qualities);
        return ptr::null_mut();
    }

    qualities
}

unsafe fn response_bytes_to_string(response: *mut GBytes) -> *mut c_char {
    let mut response_size = 0usize;
    let response_data = g_bytes_get_data(response, &mut response_size);
    g_strndup(response_data as *const c_char, response_size)
}

unsafe fn set_twitch_http_error(error: *mut *mut GError, status: c_uint) {
    if status == 401 {
        g_set_error(
            error,
            twitch_stream_info_error_quark(),
            TWITCH_STREAM_INFO_ERROR_UNAUTHORIZED,
            b"Twitch access token is expired or invalid\0".as_ptr() as *const c_char,
        );
    } else {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            b"Twitch returned HTTP %u\0".as_ptr() as *const c_char,
            status,
        );
    }
}

unsafe fn post_twitch_gql_request(
    body: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    let session = soup_session_new();
    let message = soup_message_new(
        b"POST\0".as_ptr() as *const c_char,
        TWITCH_GQL_URI.as_ptr() as *const c_char,
    );
    let body_len = CStr::from_ptr(body).to_bytes().len();
    let body_bytes = g_bytes_new_static(body as *const c_void, body_len);

    g_object_set(
        session as *mut c_void,
        b"timeout\0".as_ptr() as *const c_char,
        15,
        ptr::null::<c_char>(),
    );
    let request_headers = soup_message_get_request_headers(message);
    soup_message_headers_append(
        request_headers,
        b"Client-ID\0".as_ptr() as *const c_char,
        TWITCH_GQL_CLIENT_ID.as_ptr() as *const c_char,
    );
    soup_message_headers_append(
        request_headers,
        b"Accept\0".as_ptr() as *const c_char,
        b"application/json\0".as_ptr() as *const c_char,
    );
    soup_message_set_request_body_from_bytes(
        message,
        b"application/json\0".as_ptr() as *const c_char,
        body_bytes,
    );
    g_bytes_unref(body_bytes);

    let response = soup_session_send_and_read(session, message, cancel, error);
    if response.is_null() {
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        return ptr::null_mut();
    }

    let status = soup_message_get_status(message);
    if !(200..300).contains(&status) {
        set_twitch_http_error(error, status);
        g_bytes_unref(response);
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        return ptr::null_mut();
    }

    let result = response_bytes_to_string(response);
    g_bytes_unref(response);
    g_object_unref(message as *mut c_void);
    g_object_unref(session as *mut c_void);
    result
}

unsafe fn sanitize_oauth_token(oauth_token: *const c_char) -> *mut c_char {
    if oauth_token.is_null() {
        return g_strdup(b"\0".as_ptr() as *const c_char);
    }

    let bytes = CStr::from_ptr(oauth_token).to_bytes();
    if bytes.starts_with(b"oauth:") {
        return dup_bytes(&bytes[6..]);
    }
    if bytes.starts_with(b"Bearer ") {
        return dup_bytes(&bytes[7..]);
    }

    g_strdup(oauth_token)
}

unsafe fn get_twitch_helix_request(
    uri: *const c_char,
    client_id: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    let session = soup_session_new();
    let message = soup_message_new(b"GET\0".as_ptr() as *const c_char, uri);
    let token = sanitize_oauth_token(oauth_token);
    let mut auth = Vec::new();
    auth.extend_from_slice(b"Bearer ");
    auth.extend_from_slice(CStr::from_ptr(token).to_bytes());
    auth.push(0);

    g_object_set(
        session as *mut c_void,
        b"timeout\0".as_ptr() as *const c_char,
        15,
        ptr::null::<c_char>(),
    );
    let request_headers = soup_message_get_request_headers(message);
    soup_message_headers_append(
        request_headers,
        b"Client-ID\0".as_ptr() as *const c_char,
        client_id,
    );
    soup_message_headers_append(
        request_headers,
        b"Authorization\0".as_ptr() as *const c_char,
        auth.as_ptr() as *const c_char,
    );
    soup_message_headers_append(
        request_headers,
        b"Accept\0".as_ptr() as *const c_char,
        b"application/json\0".as_ptr() as *const c_char,
    );

    let response = soup_session_send_and_read(session, message, cancel, error);
    g_free(token as *mut c_void);
    if response.is_null() {
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        return ptr::null_mut();
    }

    let status = soup_message_get_status(message);
    if !(200..300).contains(&status) {
        set_twitch_http_error(error, status);
        g_bytes_unref(response);
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        return ptr::null_mut();
    }

    let result = response_bytes_to_string(response);
    g_bytes_unref(response);
    g_object_unref(message as *mut c_void);
    g_object_unref(session as *mut c_void);
    result
}

unsafe fn get_twitch_hls_playlist(
    uri: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut c_char {
    let session = soup_session_new();
    let message = soup_message_new(b"GET\0".as_ptr() as *const c_char, uri);

    g_object_set(
        session as *mut c_void,
        b"timeout\0".as_ptr() as *const c_char,
        15,
        ptr::null::<c_char>(),
    );
    let request_headers = soup_message_get_request_headers(message);
    soup_message_headers_append(
        request_headers,
        b"Client-ID\0".as_ptr() as *const c_char,
        TWITCH_GQL_CLIENT_ID.as_ptr() as *const c_char,
    );
    soup_message_headers_append(
        request_headers,
        b"Accept\0".as_ptr() as *const c_char,
        b"application/x-mpegURL, application/vnd.apple.mpegurl, */*\0".as_ptr() as *const c_char,
    );

    let response = soup_session_send_and_read(session, message, cancel, error);
    if response.is_null() {
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        return ptr::null_mut();
    }

    let status = soup_message_get_status(message);
    if !(200..300).contains(&status) {
        g_set_error(
            error,
            g_io_error_quark(),
            G_IO_ERROR_FAILED,
            b"Twitch HLS returned HTTP %u\0".as_ptr() as *const c_char,
            status,
        );
        g_bytes_unref(response);
        g_object_unref(message as *mut c_void);
        g_object_unref(session as *mut c_void);
        return ptr::null_mut();
    }

    let result = response_bytes_to_string(response);
    g_bytes_unref(response);
    g_object_unref(message as *mut c_void);
    g_object_unref(session as *mut c_void);
    result
}

unsafe fn parse_helix_user_id_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut c_char {
    let parser = parser_new_loaded(json, length, error);
    if parser.is_null() {
        return ptr::null_mut();
    }
    let root = json_parser_get_root(parser);
    let result = if node_is(root, JSON_NODE_OBJECT) {
        let data_node = json_object_get_member(
            json_node_get_object(root),
            b"data\0".as_ptr() as *const c_char,
        );
        if node_is(data_node, JSON_NODE_ARRAY) {
            let data = json_node_get_array(data_node);
            if json_array_get_length(data) > 0 {
                let user_node = json_array_get_element(data, 0);
                if node_is(user_node, JSON_NODE_OBJECT) {
                    let id = json_object_get_string_member_with_default(
                        json_node_get_object(user_node),
                        b"id\0".as_ptr() as *const c_char,
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

unsafe fn parse_followed_channels_page(
    json: *const c_char,
    length: usize,
    channels: *mut GPtrArray,
    cursor_out: *mut *mut c_char,
    error: *mut *mut GError,
) -> c_int {
    let parser = parser_new_loaded(json, length, error);
    if parser.is_null() {
        return 0;
    }

    let root = json_parser_get_root(parser);
    if node_is(root, JSON_NODE_OBJECT) {
        let root_object = json_node_get_object(root);
        let data_node = json_object_get_member(root_object, b"data\0".as_ptr() as *const c_char);
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
                    b"broadcaster_login\0".as_ptr() as *const c_char,
                    ptr::null(),
                );
                if !is_nonempty(login) {
                    continue;
                }
                let display_name = json_object_get_string_member_with_default(
                    channel_object,
                    b"broadcaster_name\0".as_ptr() as *const c_char,
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

        let pagination_node =
            json_object_get_member(root_object, b"pagination\0".as_ptr() as *const c_char);
        if node_is(pagination_node, JSON_NODE_OBJECT) {
            let cursor = json_object_get_string_member_with_default(
                json_node_get_object(pagination_node),
                b"cursor\0".as_ptr() as *const c_char,
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

unsafe fn fetch_current_stream(
    channel: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut TwitchCurrentStream {
    let body = build_stream_title_request_body(channel);
    let response = post_twitch_gql_request(body, cancel, error);
    g_free(body as *mut c_void);
    if response.is_null() {
        return ptr::null_mut();
    }

    let stream =
        parse_current_stream_response(response, CStr::from_ptr(response).to_bytes().len(), error);
    g_free(response as *mut c_void);
    stream
}

unsafe fn fetch_stream_qualities(
    channel: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    let body = build_playback_access_token_request_body(channel);
    let response = post_twitch_gql_request(body, cancel, error);
    g_free(body as *mut c_void);
    if response.is_null() {
        return ptr::null_mut();
    }

    let mut token: *mut c_char = ptr::null_mut();
    let mut signature: *mut c_char = ptr::null_mut();
    if parse_playback_access_token_response(
        response,
        CStr::from_ptr(response).to_bytes().len(),
        &mut token,
        &mut signature,
        error,
    ) == 0
    {
        g_free(response as *mut c_void);
        if error.is_null() || (*error).is_null() {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                b"Twitch playback token is unavailable\0".as_ptr() as *const c_char,
            );
        }
        return ptr::null_mut();
    }
    g_free(response as *mut c_void);

    let escaped_channel = g_uri_escape_string(channel, ptr::null(), 0);
    let escaped_token = g_uri_escape_string(token, ptr::null(), 0);
    let escaped_signature = g_uri_escape_string(signature, ptr::null(), 0);
    let mut uri = Vec::new();
    uri.extend_from_slice(b"https://usher.ttvnw.net/api/channel/hls/");
    uri.extend_from_slice(CStr::from_ptr(escaped_channel).to_bytes());
    uri.extend_from_slice(b".m3u8?allow_audio_only=true&allow_source=true&fast_bread=true&p=");
    uri.extend_from_slice(format!("{}", g_random_int_range(100000, 999999)).as_bytes());
    uri.extend_from_slice(b"&player=twitchweb&sig=");
    uri.extend_from_slice(CStr::from_ptr(escaped_signature).to_bytes());
    uri.extend_from_slice(b"&token=");
    uri.extend_from_slice(CStr::from_ptr(escaped_token).to_bytes());
    uri.extend_from_slice(b"&type=any\0");

    g_free(token as *mut c_void);
    g_free(signature as *mut c_void);
    g_free(escaped_channel as *mut c_void);
    g_free(escaped_token as *mut c_void);
    g_free(escaped_signature as *mut c_void);

    let playlist = get_twitch_hls_playlist(uri.as_ptr() as *const c_char, cancel, error);
    if playlist.is_null() {
        return ptr::null_mut();
    }
    let result = parse_stream_qualities_playlist(playlist, error);
    g_free(playlist as *mut c_void);
    result
}

unsafe extern "C" fn compare_stream_previews_by_viewers(
    a: *const c_void,
    b: *const c_void,
) -> c_int {
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

unsafe fn parse_live_channels_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    let parser = parser_new_loaded(json, length, error);
    if parser.is_null() {
        return ptr::null_mut();
    }
    let previews = g_ptr_array_new_with_free_func(Some(twitch_stream_preview_free_destroy));
    parse_live_channels_loaded(parser, previews);
    g_object_unref(parser as *mut c_void);
    previews
}

unsafe fn parse_live_channels_loaded(parser: *mut JsonParser, previews: *mut GPtrArray) {
    let root = json_parser_get_root(parser);
    if !node_is(root, JSON_NODE_OBJECT) {
        return;
    }
    let root_object = json_node_get_object(root);
    let data_node = json_object_get_member(root_object, b"data\0".as_ptr() as *const c_char);
    if !node_is(data_node, JSON_NODE_OBJECT) {
        return;
    }
    let users_node = json_object_get_member(
        json_node_get_object(data_node),
        b"users\0".as_ptr() as *const c_char,
    );
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
        let stream_node = json_object_get_member(user, b"stream\0".as_ptr() as *const c_char);
        if stream_node.is_null()
            || node_is(stream_node, JSON_NODE_NULL)
            || !node_is(stream_node, JSON_NODE_OBJECT)
        {
            continue;
        }

        let login = json_object_get_string_or_null(user, b"login\0".as_ptr() as *const c_char);
        if !is_nonempty(login) {
            continue;
        }

        let stream = json_node_get_object(stream_node);
        let title = json_object_get_string_or_null(stream, b"title\0".as_ptr() as *const c_char);
        let preview_url =
            json_object_get_string_or_null(stream, b"previewImageURL\0".as_ptr() as *const c_char);
        let started_at =
            json_object_get_string_or_null(stream, b"createdAt\0".as_ptr() as *const c_char);
        let display_name =
            json_object_get_string_or_null(user, b"displayName\0".as_ptr() as *const c_char);
        let avatar_url =
            json_object_get_string_or_null(user, b"profileImageURL\0".as_ptr() as *const c_char);
        let mut category_name: *const c_char = ptr::null();
        let game_node = json_object_get_member(stream, b"game\0".as_ptr() as *const c_char);
        if node_is(game_node, JSON_NODE_OBJECT) {
            category_name = json_object_get_string_or_null(
                json_node_get_object(game_node),
                b"name\0".as_ptr() as *const c_char,
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
                g_strdup(b"\0".as_ptr() as *const c_char)
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
            viewer_count: json_object_get_uint_or_zero(
                stream,
                b"viewersCount\0".as_ptr() as *const c_char,
            ),
        });
        g_ptr_array_add(previews, Box::into_raw(preview) as *mut c_void);
    }

    g_ptr_array_sort(previews, Some(compare_stream_previews_by_viewers));
}

unsafe fn fetch_live_channels(
    data: *mut FetchLiveChannelsData,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    let all_previews = g_ptr_array_new_with_free_func(Some(twitch_stream_preview_free_destroy));
    let mut offset = 0;

    while offset < (*data).channel_count {
        let chunk_count = std::cmp::min(TWITCH_GQL_MAX_LOGINS, (*data).channel_count - offset);
        let body = build_live_channels_request_body(
            (*data).channels.add(offset as usize) as *const *const c_char,
            chunk_count,
        );
        let response = post_twitch_gql_request(body, cancel, error);
        g_free(body as *mut c_void);
        if response.is_null() {
            g_ptr_array_unref(all_previews);
            return ptr::null_mut();
        }

        let chunk_previews = parse_live_channels_response(
            response,
            CStr::from_ptr(response).to_bytes().len(),
            error,
        );
        g_free(response as *mut c_void);
        if chunk_previews.is_null() {
            g_ptr_array_unref(all_previews);
            return ptr::null_mut();
        }

        for i in 0..(*chunk_previews).len {
            g_ptr_array_add(all_previews, *(*chunk_previews).pdata.add(i as usize));
        }
        g_ptr_array_set_free_func(chunk_previews, None);
        g_ptr_array_unref(chunk_previews);
        offset += TWITCH_GQL_MAX_LOGINS;
    }

    g_ptr_array_sort(all_previews, Some(compare_stream_previews_by_viewers));
    all_previews
}

unsafe extern "C" fn fetch_current_stream_data_free(data: *mut c_void) {
    let data = data as *mut FetchCurrentStreamData;
    if data.is_null() {
        return;
    }
    g_free((*data).channel as *mut c_void);
    drop(Box::from_raw(data));
}

unsafe extern "C" fn fetch_stream_qualities_data_free(data: *mut c_void) {
    let data = data as *mut FetchStreamQualitiesData;
    if data.is_null() {
        return;
    }
    g_free((*data).channel as *mut c_void);
    drop(Box::from_raw(data));
}

unsafe extern "C" fn fetch_live_channels_data_free(data: *mut c_void) {
    let data = data as *mut FetchLiveChannelsData;
    if data.is_null() {
        return;
    }
    if !(*data).channels.is_null() {
        for i in 0..(*data).channel_count {
            g_free(*(*data).channels.add(i as usize) as *mut c_void);
        }
        g_free((*data).channels as *mut c_void);
    }
    drop(Box::from_raw(data));
}

unsafe extern "C" fn fetch_followed_channels_data_free(data: *mut c_void) {
    let data = data as *mut FetchFollowedChannelsData;
    if data.is_null() {
        return;
    }
    g_free((*data).client_id as *mut c_void);
    g_free((*data).oauth_token as *mut c_void);
    drop(Box::from_raw(data));
}

unsafe extern "C" fn fetch_current_stream_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    let data = task_data as *mut FetchCurrentStreamData;
    let mut error: *mut GError = ptr::null_mut();
    let stream = fetch_current_stream((*data).channel, cancel, &mut error);
    if !error.is_null() {
        g_task_return_error(task, error);
        return;
    }
    g_task_return_pointer(
        task,
        stream as *mut c_void,
        Some(twitch_current_stream_free_destroy),
    );
}

unsafe extern "C" fn fetch_stream_qualities_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    let data = task_data as *mut FetchStreamQualitiesData;
    let mut error: *mut GError = ptr::null_mut();
    let qualities = fetch_stream_qualities((*data).channel, cancel, &mut error);
    if !error.is_null() {
        if !qualities.is_null() {
            g_ptr_array_unref(qualities);
        }
        g_task_return_error(task, error);
        return;
    }
    g_task_return_pointer(
        task,
        qualities as *mut c_void,
        Some(g_ptr_array_unref_destroy),
    );
}

unsafe extern "C" fn fetch_live_channels_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    let data = task_data as *mut FetchLiveChannelsData;
    let mut error: *mut GError = ptr::null_mut();
    let previews = fetch_live_channels(data, cancel, &mut error);
    if !error.is_null() {
        g_task_return_error(task, error);
        return;
    }
    g_task_return_pointer(
        task,
        previews as *mut c_void,
        Some(g_ptr_array_unref_destroy),
    );
}

unsafe extern "C" fn fetch_followed_channels_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    cancel: *mut GCancellable,
) {
    let data = task_data as *mut FetchFollowedChannelsData;
    let mut error: *mut GError = ptr::null_mut();
    let channels = twitch_stream_info_fetch_followed_channels(
        (*data).client_id,
        (*data).oauth_token,
        cancel,
        &mut error,
    );
    if !error.is_null() {
        g_task_return_error(task, error);
        return;
    }
    g_task_return_pointer(
        task,
        channels as *mut c_void,
        Some(g_ptr_array_unref_destroy),
    );
}

unsafe extern "C" fn g_ptr_array_unref_destroy(data: *mut c_void) {
    g_ptr_array_unref(data as *mut GPtrArray);
}

pub unsafe fn twitch_stream_info_error_quark() -> c_uint {
    g_quark_from_static_string(b"twitch-stream-info-error\0".as_ptr() as *const c_char)
}

unsafe extern "C" fn twitch_stream_preview_free_destroy(data: *mut c_void) {
    twitch_stream_preview_free(data as *mut TwitchStreamPreview);
}

unsafe extern "C" fn twitch_current_stream_free_destroy(data: *mut c_void) {
    twitch_current_stream_free(data as *mut TwitchCurrentStream);
}

pub unsafe fn twitch_stream_preview_free(preview: *mut TwitchStreamPreview) {
    if preview.is_null() {
        return;
    }
    g_free((*preview).channel as *mut c_void);
    g_free((*preview).display_name as *mut c_void);
    g_free((*preview).title as *mut c_void);
    g_free((*preview).avatar_url as *mut c_void);
    g_free((*preview).preview_url as *mut c_void);
    g_free((*preview).started_at as *mut c_void);
    g_free((*preview).category_name as *mut c_void);
    drop(Box::from_raw(preview));
}

pub unsafe fn twitch_current_stream_free(stream: *mut TwitchCurrentStream) {
    if stream.is_null() {
        return;
    }
    g_free((*stream).title as *mut c_void);
    g_free((*stream).started_at as *mut c_void);
    g_free((*stream).category_name as *mut c_void);
    drop(Box::from_raw(stream));
}

unsafe extern "C" fn twitch_stream_quality_free_destroy(data: *mut c_void) {
    twitch_stream_quality_free(data as *mut TwitchStreamQuality);
}

pub unsafe fn twitch_stream_quality_free(quality: *mut TwitchStreamQuality) {
    if quality.is_null() {
        return;
    }
    g_free((*quality).label as *mut c_void);
    g_free((*quality).url as *mut c_void);
    drop(Box::from_raw(quality));
}

pub unsafe fn twitch_followed_channel_free(channel: *mut TwitchFollowedChannel) {
    if channel.is_null() {
        return;
    }
    g_free((*channel).channel as *mut c_void);
    g_free((*channel).display_name as *mut c_void);
    drop(Box::from_raw(channel));
}

pub unsafe fn twitch_stream_info_format_viewer_count(viewer_count: c_uint) -> *mut c_char {
    let text = if viewer_count >= 1_000_000 {
        format!("{:.1}M", viewer_count as f64 / 1_000_000.0)
    } else if viewer_count >= 1000 {
        format!("{:.1}K", viewer_count as f64 / 1000.0)
    } else {
        viewer_count.to_string()
    };
    dup_bytes(text.as_bytes())
}

unsafe fn format_live_duration_from_span(span: i64) -> *mut c_char {
    let span = span.max(0);
    let total_minutes = span / 60_000_000;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let text = if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    };
    dup_bytes(text.as_bytes())
}

pub unsafe fn twitch_stream_info_format_live_duration(started_at: *const c_char) -> *mut c_char {
    if !is_nonempty(started_at) {
        return ptr::null_mut();
    }

    let started = g_date_time_new_from_iso8601(started_at, ptr::null_mut());
    if started.is_null() {
        return ptr::null_mut();
    }
    let now = g_date_time_new_now_utc();
    let result = format_live_duration_from_span(g_date_time_difference(now, started));
    g_date_time_unref(now);
    g_date_time_unref(started);
    result
}

pub unsafe fn twitch_stream_info_format_current_stream_title(
    stream: *const TwitchCurrentStream,
) -> *mut c_char {
    if stream.is_null() || !is_nonempty((*stream).title) {
        return g_strdup(b"\0".as_ptr() as *const c_char);
    }
    g_strdup((*stream).title)
}

unsafe fn append_metadata_segment(metadata: &mut Vec<u8>, segment: *const c_char) {
    if !is_nonempty(segment) {
        return;
    }
    if !metadata.is_empty() {
        metadata.extend_from_slice(b" \xE2\x80\xA2 ");
    }
    metadata.extend_from_slice(CStr::from_ptr(segment).to_bytes());
}

pub unsafe fn twitch_stream_info_format_current_stream_metadata(
    stream: *const TwitchCurrentStream,
) -> *mut c_char {
    if stream.is_null() {
        return g_strdup(b"\0".as_ptr() as *const c_char);
    }

    let viewers = twitch_stream_info_format_viewer_count((*stream).viewer_count);
    let duration = twitch_stream_info_format_live_duration((*stream).started_at);
    let mut metadata = Vec::new();
    append_metadata_segment(&mut metadata, viewers);
    append_metadata_segment(&mut metadata, duration);
    append_metadata_segment(&mut metadata, (*stream).category_name);
    g_free(viewers as *mut c_void);
    g_free(duration as *mut c_void);
    dup_bytes(&metadata)
}

pub unsafe fn twitch_stream_info_fetch_current_stream_async(
    channel: *const c_char,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    if !is_nonempty(channel) {
        return;
    }
    let data = Box::into_raw(Box::new(FetchCurrentStreamData {
        channel: g_ascii_strdown(channel, -1),
    }));
    let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
    g_task_set_task_data(
        task,
        data as *mut c_void,
        Some(fetch_current_stream_data_free),
    );
    g_task_run_in_thread(task, Some(fetch_current_stream_worker));
    g_object_unref(task as *mut c_void);
}

pub unsafe fn twitch_stream_info_fetch_current_stream_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut TwitchCurrentStream {
    if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
        return ptr::null_mut();
    }
    g_task_propagate_pointer(result as *mut GTask, error) as *mut TwitchCurrentStream
}

pub unsafe fn twitch_stream_info_fetch_stream_qualities_async(
    channel: *const c_char,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    if !is_nonempty(channel) {
        return;
    }
    let data = Box::into_raw(Box::new(FetchStreamQualitiesData {
        channel: g_ascii_strdown(channel, -1),
    }));
    let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
    g_task_set_task_data(
        task,
        data as *mut c_void,
        Some(fetch_stream_qualities_data_free),
    );
    g_task_run_in_thread(task, Some(fetch_stream_qualities_worker));
    g_object_unref(task as *mut c_void);
}

pub unsafe fn twitch_stream_info_fetch_stream_qualities_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
        return ptr::null_mut();
    }
    g_task_propagate_pointer(result as *mut GTask, error) as *mut GPtrArray
}

pub unsafe fn twitch_stream_info_fetch_live_channels_async(
    channels: *const *const c_char,
    channel_count: c_uint,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    let data = Box::into_raw(Box::new(FetchLiveChannelsData {
        channel_count,
        channels: g_malloc0((channel_count as usize + 1) * std::mem::size_of::<*mut c_char>())
            as *mut *mut c_char,
    }));
    for i in 0..channel_count {
        let channel = *channels.add(i as usize);
        *(*data).channels.add(i as usize) = if channel.is_null() {
            g_strdup(b"\0".as_ptr() as *const c_char)
        } else {
            g_ascii_strdown(channel, -1)
        };
    }
    let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
    g_task_set_task_data(
        task,
        data as *mut c_void,
        Some(fetch_live_channels_data_free),
    );
    g_task_run_in_thread(task, Some(fetch_live_channels_worker));
    g_object_unref(task as *mut c_void);
}

pub unsafe fn twitch_stream_info_fetch_live_channels_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
        return ptr::null_mut();
    }
    g_task_propagate_pointer(result as *mut GTask, error) as *mut GPtrArray
}

pub unsafe fn twitch_stream_info_fetch_followed_channels(
    client_id: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    error: *mut *mut GError,
) -> *mut GPtrArray {
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
                b"Twitch user could not be read from the token\0".as_ptr() as *const c_char,
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
        request_uri.extend_from_slice(b"https://api.twitch.tv/helix/channels/followed?user_id=");
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

unsafe extern "C" {
    fn g_cancellable_is_cancelled(cancellable: *mut GCancellable) -> c_int;
}

unsafe extern "C" fn twitch_followed_channel_free_destroy(data: *mut c_void) {
    twitch_followed_channel_free(data as *mut TwitchFollowedChannel);
}

pub unsafe fn twitch_stream_info_fetch_followed_channels_async(
    client_id: *const c_char,
    oauth_token: *const c_char,
    cancel: *mut GCancellable,
    callback: Option<GAsyncReadyCallback>,
    user_data: *mut c_void,
) {
    if !is_nonempty(client_id) || !is_nonempty(oauth_token) {
        return;
    }
    let data = Box::into_raw(Box::new(FetchFollowedChannelsData {
        client_id: g_strdup(client_id),
        oauth_token: g_strdup(oauth_token),
    }));
    let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
    g_task_set_task_data(
        task,
        data as *mut c_void,
        Some(fetch_followed_channels_data_free),
    );
    g_task_run_in_thread(task, Some(fetch_followed_channels_worker));
    g_object_unref(task as *mut c_void);
}

pub unsafe fn twitch_stream_info_fetch_followed_channels_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    if g_task_is_valid(result as *mut c_void, ptr::null_mut()) == 0 {
        return ptr::null_mut();
    }
    g_task_propagate_pointer(result as *mut GTask, error) as *mut GPtrArray
}

pub unsafe fn twitch_stream_info_test_build_stream_title_request_body(
    channel: *const c_char,
) -> *mut c_char {
    build_stream_title_request_body(channel)
}

pub unsafe fn twitch_stream_info_test_build_live_channels_request_body(
    channels: *const *const c_char,
    channel_count: c_uint,
) -> *mut c_char {
    build_live_channels_request_body(channels, channel_count)
}

pub unsafe fn twitch_stream_info_test_parse_current_stream_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut TwitchCurrentStream {
    parse_current_stream_response(json, length, error)
}

pub unsafe fn twitch_stream_info_test_parse_stream_qualities_playlist(
    playlist: *const c_char,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    parse_stream_qualities_playlist(playlist, error)
}

pub unsafe fn twitch_stream_info_test_format_live_duration_from_span(span: i64) -> *mut c_char {
    format_live_duration_from_span(span)
}

pub unsafe fn twitch_stream_info_test_parse_live_channels_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut GPtrArray {
    parse_live_channels_response(json, length, error)
}

pub unsafe fn twitch_stream_info_test_parse_helix_user_id_response(
    json: *const c_char,
    length: usize,
    error: *mut *mut GError,
) -> *mut c_char {
    parse_helix_user_id_response(json, length, error)
}

pub unsafe fn twitch_stream_info_test_parse_followed_channels_page(
    json: *const c_char,
    length: usize,
    channels: *mut GPtrArray,
    cursor_out: *mut *mut c_char,
    error: *mut *mut GError,
) -> c_int {
    parse_followed_channels_page(json, length, channels, cursor_out, error)
}
