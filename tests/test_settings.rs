use std::env;
use std::ffi::{c_char, CStr, CString};
use std::fs;
use std::path::PathBuf;
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};
use twitch_player_core::settings;

unsafe extern "C" {
    fn g_clear_error(error: *mut *mut settings::GError);
}

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn assert_cstr_eq(actual: *const c_char, expected: &str) {
    assert!(!actual.is_null());
    assert_eq!(CStr::from_ptr(actual).to_str().unwrap(), expected);
}

fn temp_config_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!(
        "twitch-player-settings-{}-{nanos}",
        std::process::id()
    ))
}

unsafe fn test_settings_round_trip_channels() {
    let config_dir = temp_config_dir();
    fs::create_dir(&config_dir).unwrap();
    env::set_var("XDG_CONFIG_HOME", &config_dir);

    let mut error: *mut settings::GError = ptr::null_mut();
    let mut settings = settings::app_settings_new();
    assert_ne!(settings::app_settings_get_hwdec_enabled(settings), 0);
    settings::app_settings_set_hwdec_enabled(settings, 0);
    settings::app_settings_set_twitch_auth_tokens(
        settings,
        cstring("token-123").as_ptr(),
        cstring("refresh-456").as_ptr(),
        123456789,
    );
    settings::app_settings_add_channel(
        settings,
        cstring("Papaplatte Live").as_ptr(),
        cstring("https://www.twitch.tv/PapaPlatte").as_ptr(),
        ptr::null(),
    );

    assert_ne!(settings::app_settings_save(settings, &mut error), 0);
    assert!(error.is_null());
    settings::app_settings_free(settings);

    settings = settings::app_settings_load();
    assert_eq!(settings::app_settings_get_hwdec_enabled(settings), 0);
    assert_cstr_eq(
        settings::app_settings_get_twitch_oauth_token(settings),
        "token-123",
    );
    assert_cstr_eq(
        settings::app_settings_get_twitch_refresh_token(settings),
        "refresh-456",
    );
    assert_eq!(
        settings::app_settings_get_twitch_oauth_expires_at(settings),
        123456789
    );
    assert_eq!(settings::app_settings_get_channel_count(settings), 1);

    let channel = settings::app_settings_get_channel(settings, 0);
    assert!(!channel.is_null());
    assert_cstr_eq((*channel).label, "Papaplatte Live");
    assert_cstr_eq((*channel).channel, "papaplatte");
    assert_cstr_eq((*channel).url, "https://www.twitch.tv/papaplatte");
    settings::app_settings_free(settings);

    if !error.is_null() {
        g_clear_error(&mut error);
    }
    fs::remove_dir_all(&config_dir).unwrap();
}

fn main() {
    unsafe {
        test_settings_round_trip_channels();
    }
}
