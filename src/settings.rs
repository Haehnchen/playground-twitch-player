use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::io;
use std::ptr;

const G_FILE_TEST_EXISTS: c_int = 1 << 4;
const JSON_NODE_OBJECT: c_int = 0;
const JSON_NODE_ARRAY: c_int = 1;

pub struct AppSettingsChannel {
    pub label: *mut c_char,
    pub channel: *mut c_char,
    pub url: *mut c_char,
}

pub struct AppSettings {
    channels: *mut GPtrArray,
    twitch_oauth_token: *mut c_char,
    twitch_refresh_token: *mut c_char,
    twitch_oauth_expires_at: i64,
    hwdec_enabled: c_int,
}

#[repr(C)]
pub struct GPtrArray {
    pdata: *mut *mut c_void,
    len: c_uint,
}

#[repr(C)]
pub struct GError {
    pub domain: c_uint,
    pub code: c_int,
    pub message: *mut c_char,
}

#[repr(C)]
pub struct JsonParser {
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
pub struct JsonArray {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonBuilder {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonGenerator {
    _private: [u8; 0],
}

type GDestroyNotify = unsafe extern "C" fn(*mut c_void);

unsafe extern "C" {
    fn g_build_filenamev(args: *mut *mut c_char) -> *mut c_char;
    fn g_clear_error(error: *mut *mut GError);
    fn g_file_test(filename: *const c_char, test: c_int) -> c_int;
    fn g_file_error_from_errno(err_no: c_int) -> c_int;
    fn g_file_error_quark() -> c_uint;
    fn g_free(mem: *mut c_void);
    fn g_get_user_config_dir() -> *const c_char;
    fn g_mkdir_with_parents(pathname: *const c_char, mode: c_int) -> c_int;
    fn g_object_unref(object: *mut c_void);
    fn g_ptr_array_add(array: *mut GPtrArray, data: *mut c_void);
    fn g_ptr_array_new_with_free_func(element_free_func: Option<GDestroyNotify>) -> *mut GPtrArray;
    fn g_ptr_array_set_size(array: *mut GPtrArray, length: c_uint);
    fn g_ptr_array_unref(array: *mut GPtrArray);
    fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_strerror(errnum: c_int) -> *const c_char;

    fn json_array_get_element(array: *mut JsonArray, index_: c_uint) -> *mut JsonNode;
    fn json_array_get_length(array: *mut JsonArray) -> c_uint;
    fn json_builder_add_boolean_value(builder: *mut JsonBuilder, value: c_int) -> *mut JsonBuilder;
    fn json_builder_add_int_value(builder: *mut JsonBuilder, value: i64) -> *mut JsonBuilder;
    fn json_builder_add_string_value(
        builder: *mut JsonBuilder,
        value: *const c_char,
    ) -> *mut JsonBuilder;
    fn json_builder_begin_array(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    fn json_builder_begin_object(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    fn json_builder_end_array(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    fn json_builder_end_object(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    fn json_builder_get_root(builder: *mut JsonBuilder) -> *mut JsonNode;
    fn json_builder_new() -> *mut JsonBuilder;
    fn json_builder_set_member_name(
        builder: *mut JsonBuilder,
        member_name: *const c_char,
    ) -> *mut JsonBuilder;
    fn json_generator_new() -> *mut JsonGenerator;
    fn json_generator_set_pretty(generator: *mut JsonGenerator, is_pretty: c_int);
    fn json_generator_set_root(generator: *mut JsonGenerator, node: *mut JsonNode);
    fn json_generator_to_file(
        generator: *mut JsonGenerator,
        filename: *const c_char,
        error: *mut *mut GError,
    ) -> c_int;
    fn json_node_get_array(node: *mut JsonNode) -> *mut JsonArray;
    fn json_node_get_node_type(node: *mut JsonNode) -> c_int;
    fn json_node_get_object(node: *mut JsonNode) -> *mut JsonObject;
    fn json_node_unref(node: *mut JsonNode);
    fn json_object_get_boolean_member_with_default(
        object: *mut JsonObject,
        member_name: *const c_char,
        default_value: c_int,
    ) -> c_int;
    fn json_object_get_int_member_with_default(
        object: *mut JsonObject,
        member_name: *const c_char,
        default_value: i64,
    ) -> i64;
    fn json_object_get_member(object: *mut JsonObject, member_name: *const c_char)
        -> *mut JsonNode;
    fn json_object_get_string_member_with_default(
        object: *mut JsonObject,
        member_name: *const c_char,
        default_value: *const c_char,
    ) -> *const c_char;
    fn json_parser_get_root(parser: *mut JsonParser) -> *mut JsonNode;
    fn json_parser_load_from_file(
        parser: *mut JsonParser,
        filename: *const c_char,
        error: *mut *mut GError,
    ) -> c_int;
    fn json_parser_new() -> *mut JsonParser;
}

unsafe extern "C" fn app_settings_channel_free(data: *mut c_void) {
    if data.is_null() {
        return;
    }

    let channel = Box::from_raw(data as *mut AppSettingsChannel);
    g_free(channel.label as *mut c_void);
    g_free(channel.channel as *mut c_void);
    g_free(channel.url as *mut c_void);
}

unsafe fn c_bytes(value: *const c_char) -> Vec<u8> {
    if value.is_null() {
        Vec::new()
    } else {
        CStr::from_ptr(value).to_bytes().to_vec()
    }
}

unsafe fn trimmed_bytes(value: *const c_char) -> Vec<u8> {
    let bytes = c_bytes(value);
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    bytes[start..end].to_vec()
}

unsafe fn dup_bytes(bytes: &[u8]) -> *mut c_char {
    let string = CString::new(bytes).unwrap_or_default();
    g_strdup(string.as_ptr())
}

unsafe fn is_nonempty(value: *const c_char) -> bool {
    !value.is_null() && *value != 0
}

fn extract_twitch_channel_name(value: &[u8]) -> Option<Vec<u8>> {
    if value.is_empty() {
        return None;
    }

    let needle = b"twitch.tv/";
    let mut start = value
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|position| position + needle.len())
        .unwrap_or(0);

    while start < value.len() && (value[start] == b'/' || value[start] == b'@') {
        start += 1;
    }

    let mut end = start;
    while end < value.len() && (value[end].is_ascii_alphanumeric() || value[end] == b'_') {
        end += 1;
    }

    if end == start {
        return None;
    }

    Some(
        value[start..end]
            .iter()
            .map(u8::to_ascii_lowercase)
            .collect(),
    )
}

unsafe fn build_filename(parts: &[*const c_char]) -> *mut c_char {
    let mut args: Vec<*mut c_char> = parts.iter().map(|part| *part as *mut c_char).collect();
    args.push(ptr::null_mut());
    g_build_filenamev(args.as_mut_ptr())
}

pub unsafe fn app_settings_new() -> *mut AppSettings {
    Box::into_raw(Box::new(AppSettings {
        channels: g_ptr_array_new_with_free_func(Some(app_settings_channel_free)),
        twitch_oauth_token: ptr::null_mut(),
        twitch_refresh_token: ptr::null_mut(),
        twitch_oauth_expires_at: 0,
        hwdec_enabled: 1,
    }))
}

pub unsafe fn app_settings_get_path() -> *mut c_char {
    build_filename(&[
        g_get_user_config_dir(),
        b"twitch-player\0".as_ptr() as *const c_char,
        b"settings.json\0".as_ptr() as *const c_char,
    ])
}

pub unsafe fn app_settings_free(settings: *mut AppSettings) {
    if settings.is_null() {
        return;
    }

    let settings = Box::from_raw(settings);
    g_ptr_array_unref(settings.channels);
    g_free(settings.twitch_oauth_token as *mut c_void);
    g_free(settings.twitch_refresh_token as *mut c_void);
}

pub unsafe fn app_settings_get_channel_count(settings: *const AppSettings) -> c_uint {
    if settings.is_null() {
        return 0;
    }

    (*(*settings).channels).len
}

pub unsafe fn app_settings_get_channel(
    settings: *const AppSettings,
    index: c_uint,
) -> *const AppSettingsChannel {
    if settings.is_null() || index >= (*(*settings).channels).len {
        return ptr::null();
    }

    *(*(*settings).channels).pdata.add(index as usize) as *const AppSettingsChannel
}

pub unsafe fn app_settings_get_hwdec_enabled(settings: *const AppSettings) -> c_int {
    if settings.is_null() {
        return 1;
    }

    (*settings).hwdec_enabled
}

pub unsafe fn app_settings_set_hwdec_enabled(settings: *mut AppSettings, enabled: c_int) {
    if !settings.is_null() {
        (*settings).hwdec_enabled = enabled;
    }
}

pub unsafe fn app_settings_get_twitch_oauth_token(settings: *const AppSettings) -> *const c_char {
    if settings.is_null() {
        return ptr::null();
    }

    (*settings).twitch_oauth_token
}

pub unsafe fn app_settings_get_twitch_refresh_token(settings: *const AppSettings) -> *const c_char {
    if settings.is_null() {
        return ptr::null();
    }

    (*settings).twitch_refresh_token
}

pub unsafe fn app_settings_get_twitch_oauth_expires_at(settings: *const AppSettings) -> i64 {
    if settings.is_null() {
        return 0;
    }

    (*settings).twitch_oauth_expires_at
}

pub unsafe fn app_settings_set_twitch_oauth_token(
    settings: *mut AppSettings,
    oauth_token: *const c_char,
) {
    if settings.is_null() {
        return;
    }

    let new_oauth_token = if is_nonempty(oauth_token) {
        g_strdup(oauth_token)
    } else {
        ptr::null_mut()
    };
    g_free((*settings).twitch_oauth_token as *mut c_void);
    (*settings).twitch_oauth_token = new_oauth_token;
    if (*settings).twitch_oauth_token.is_null() {
        g_free((*settings).twitch_refresh_token as *mut c_void);
        (*settings).twitch_refresh_token = ptr::null_mut();
        (*settings).twitch_oauth_expires_at = 0;
    }
}

pub unsafe fn app_settings_set_twitch_auth_tokens(
    settings: *mut AppSettings,
    oauth_token: *const c_char,
    refresh_token: *const c_char,
    oauth_expires_at: i64,
) {
    if settings.is_null() {
        return;
    }

    let new_oauth_token = if is_nonempty(oauth_token) {
        g_strdup(oauth_token)
    } else {
        ptr::null_mut()
    };
    let new_refresh_token = if is_nonempty(refresh_token) {
        g_strdup(refresh_token)
    } else {
        ptr::null_mut()
    };

    g_free((*settings).twitch_oauth_token as *mut c_void);
    (*settings).twitch_oauth_token = new_oauth_token;
    g_free((*settings).twitch_refresh_token as *mut c_void);
    (*settings).twitch_refresh_token = new_refresh_token;
    (*settings).twitch_oauth_expires_at = if (*settings).twitch_oauth_token.is_null() {
        0
    } else {
        oauth_expires_at
    };
}

pub unsafe fn app_settings_clear_channels(settings: *mut AppSettings) {
    if !settings.is_null() {
        g_ptr_array_set_size((*settings).channels, 0);
    }
}

pub unsafe fn app_settings_add_channel(
    settings: *mut AppSettings,
    label: *const c_char,
    channel: *const c_char,
    url: *const c_char,
) {
    if settings.is_null() {
        return;
    }

    let trimmed_label = trimmed_bytes(label);
    let trimmed_channel = trimmed_bytes(channel);
    let trimmed_url = trimmed_bytes(url);
    let derived_channel = extract_twitch_channel_name(&trimmed_channel)
        .or_else(|| extract_twitch_channel_name(&trimmed_url));

    if derived_channel.is_none() && trimmed_url.is_empty() && trimmed_channel.is_empty() {
        return;
    }

    let stored_channel = derived_channel.unwrap_or(trimmed_channel);
    let stored_url = if !trimmed_url.is_empty() {
        trimmed_url
    } else if !stored_channel.is_empty() {
        let mut value = b"https://www.twitch.tv/".to_vec();
        value.extend_from_slice(&stored_channel);
        value
    } else {
        Vec::new()
    };
    let stored_label = if !trimmed_label.is_empty() {
        trimmed_label
    } else if !stored_channel.is_empty() {
        stored_channel.clone()
    } else {
        stored_url.clone()
    };

    let entry = Box::new(AppSettingsChannel {
        label: dup_bytes(&stored_label),
        channel: dup_bytes(&stored_channel),
        url: dup_bytes(&stored_url),
    });
    g_ptr_array_add((*settings).channels, Box::into_raw(entry) as *mut c_void);
}

unsafe fn load_channels(settings: *mut AppSettings, root: *mut JsonObject) {
    let channels_node = json_object_get_member(root, b"channels\0".as_ptr() as *const c_char);
    if channels_node.is_null() || json_node_get_node_type(channels_node) != JSON_NODE_ARRAY {
        return;
    }

    let channels = json_node_get_array(channels_node);
    for i in 0..json_array_get_length(channels) {
        let node = json_array_get_element(channels, i);
        if node.is_null() || json_node_get_node_type(node) != JSON_NODE_OBJECT {
            continue;
        }

        let item = json_node_get_object(node);
        let label = json_object_get_string_member_with_default(
            item,
            b"label\0".as_ptr() as *const c_char,
            b"\0".as_ptr() as *const c_char,
        );
        let channel = json_object_get_string_member_with_default(
            item,
            b"channel\0".as_ptr() as *const c_char,
            b"\0".as_ptr() as *const c_char,
        );
        let url = json_object_get_string_member_with_default(
            item,
            b"url\0".as_ptr() as *const c_char,
            b"\0".as_ptr() as *const c_char,
        );
        app_settings_add_channel(settings, label, channel, url);
    }
}

pub unsafe fn app_settings_load() -> *mut AppSettings {
    let settings = app_settings_new();
    let path = app_settings_get_path();

    if g_file_test(path, G_FILE_TEST_EXISTS) == 0 {
        g_free(path as *mut c_void);
        return settings;
    }

    let parser = json_parser_new();
    let mut error: *mut GError = ptr::null_mut();
    if json_parser_load_from_file(parser, path, &mut error) == 0 {
        g_clear_error(&mut error);
        g_object_unref(parser as *mut c_void);
        g_free(path as *mut c_void);
        return settings;
    }

    let root_node = json_parser_get_root(parser);
    if root_node.is_null() || json_node_get_node_type(root_node) != JSON_NODE_OBJECT {
        g_object_unref(parser as *mut c_void);
        g_free(path as *mut c_void);
        return settings;
    }

    let root = json_node_get_object(root_node);
    (*settings).hwdec_enabled =
        json_object_get_boolean_member_with_default(root, b"hwdec\0".as_ptr() as *const c_char, 1);
    app_settings_set_twitch_oauth_token(
        settings,
        json_object_get_string_member_with_default(
            root,
            b"twitch_oauth_token\0".as_ptr() as *const c_char,
            ptr::null(),
        ),
    );
    app_settings_set_twitch_auth_tokens(
        settings,
        app_settings_get_twitch_oauth_token(settings),
        json_object_get_string_member_with_default(
            root,
            b"twitch_refresh_token\0".as_ptr() as *const c_char,
            ptr::null(),
        ),
        json_object_get_int_member_with_default(
            root,
            b"twitch_oauth_expires_at\0".as_ptr() as *const c_char,
            0,
        ),
    );
    load_channels(settings, root);

    g_object_unref(parser as *mut c_void);
    g_free(path as *mut c_void);
    settings
}

pub unsafe fn app_settings_save<E>(settings: *mut AppSettings, error: *mut *mut E) -> c_int {
    let error = error as *mut *mut GError;
    let config_dir = build_filename(&[
        g_get_user_config_dir(),
        b"twitch-player\0".as_ptr() as *const c_char,
    ]);
    let path = app_settings_get_path();

    if g_mkdir_with_parents(config_dir, 0o700) < 0 {
        let errno = io::Error::last_os_error().raw_os_error().unwrap_or(0);
        g_set_error(
            error,
            g_file_error_quark(),
            g_file_error_from_errno(errno),
            b"Could not create %s: %s\0".as_ptr() as *const c_char,
            config_dir,
            g_strerror(errno),
        );
        g_free(config_dir as *mut c_void);
        g_free(path as *mut c_void);
        return 0;
    }

    let builder = json_builder_new();
    json_builder_begin_object(builder);
    json_builder_set_member_name(builder, b"hwdec\0".as_ptr() as *const c_char);
    json_builder_add_boolean_value(builder, app_settings_get_hwdec_enabled(settings));

    let oauth_token = app_settings_get_twitch_oauth_token(settings);
    if is_nonempty(oauth_token) {
        json_builder_set_member_name(builder, b"twitch_oauth_token\0".as_ptr() as *const c_char);
        json_builder_add_string_value(builder, oauth_token);
    }
    let refresh_token = app_settings_get_twitch_refresh_token(settings);
    if is_nonempty(refresh_token) {
        json_builder_set_member_name(builder, b"twitch_refresh_token\0".as_ptr() as *const c_char);
        json_builder_add_string_value(builder, refresh_token);
    }
    let oauth_expires_at = app_settings_get_twitch_oauth_expires_at(settings);
    if oauth_expires_at > 0 {
        json_builder_set_member_name(
            builder,
            b"twitch_oauth_expires_at\0".as_ptr() as *const c_char,
        );
        json_builder_add_int_value(builder, oauth_expires_at);
    }

    json_builder_set_member_name(builder, b"channels\0".as_ptr() as *const c_char);
    json_builder_begin_array(builder);
    for i in 0..app_settings_get_channel_count(settings) {
        let channel = app_settings_get_channel(settings, i);
        json_builder_begin_object(builder);
        json_builder_set_member_name(builder, b"label\0".as_ptr() as *const c_char);
        json_builder_add_string_value(builder, (*channel).label);
        json_builder_set_member_name(builder, b"channel\0".as_ptr() as *const c_char);
        json_builder_add_string_value(builder, (*channel).channel);
        json_builder_set_member_name(builder, b"url\0".as_ptr() as *const c_char);
        json_builder_add_string_value(builder, (*channel).url);
        json_builder_end_object(builder);
    }
    json_builder_end_array(builder);
    json_builder_end_object(builder);

    let root = json_builder_get_root(builder);
    let generator = json_generator_new();
    json_generator_set_root(generator, root);
    json_generator_set_pretty(generator, 1);
    let result = json_generator_to_file(generator, path, error);

    g_object_unref(generator as *mut c_void);
    json_node_unref(root);
    g_object_unref(builder as *mut c_void);
    g_free(config_dir as *mut c_void);
    g_free(path as *mut c_void);
    result
}
