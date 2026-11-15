use super::ffi::{GTask, g_task_return_pointer};
use super::{ChannelListResult, FetchChannelListData, FollowedCache, drop_box};
use crate::settings::{
    AppSettings, app_settings_get_channel, app_settings_get_channel_count, app_settings_save,
    app_settings_set_twitch_auth_tokens,
};
use crate::twitch::auth::{TwitchAuthToken, twitch_auth_refresh_token};
use crate::twitch::stream_info::{
    GCancellable, GError, TwitchFollowedChannel, twitch_stream_info_error_quark,
    twitch_stream_info_fetch_followed_channels,
};
use gtk4::{
    gio,
    glib::{self, translate::*},
};
use std::{
    collections::HashSet,
    ffi::{CString, c_uint, c_void},
    ptr,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const TWITCH_AUTH_CLIENT_ID: &std::ffi::CStr = c"8l1fzyh4jhs1cxhtqs6p4swmxuejh6";
const FOLLOWED_CHANNELS_CACHE_SECONDS: u64 = 120;

fn token_expires_at_from_expires_in(expires_in: c_uint) -> i64 {
    if expires_in > 0 {
        glib::real_time() / 1_000_000 + expires_in as i64
    } else {
        0
    }
}

fn token_needs_refresh(expires_at: i64) -> bool {
    expires_at > 0 && glib::real_time() / 1_000_000 + 60 >= expires_at
}

// Keep configured channels first and ignore duplicate logins.
fn unique_channels(channels: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    channels
        .into_iter()
        .map(|channel| channel.to_ascii_lowercase())
        .filter(|channel| !channel.is_empty() && seen.insert(channel.clone()))
        .collect()
}

pub(super) unsafe fn collect_settings_channels(settings: *const AppSettings) -> Vec<String> {
    unsafe {
        (0..app_settings_get_channel_count(settings))
            .filter_map(|index| {
                let channel = &*app_settings_get_channel(settings, index);
                (!channel.channel.is_empty())
                    .then(|| channel.channel.to_string_lossy().into_owned())
            })
            .collect()
    }
}

unsafe fn refresh_access_token(
    data: &FetchChannelListData,
    cancel: *mut GCancellable,
) -> Result<Box<TwitchAuthToken>, glib::Error> {
    unsafe {
        if data.refresh_token.is_empty() {
            return Err(glib::Error::new(
                gio::IOErrorEnum::Failed,
                "Connect Twitch again to refresh followed channels",
            ));
        }
        let mut error = ptr::null_mut();
        let token = twitch_auth_refresh_token(
            TWITCH_AUTH_CLIENT_ID.as_ptr(),
            data.refresh_token.as_ptr(),
            cancel,
            &mut error,
        );
        if !error.is_null() {
            return Err(from_glib_full(error));
        }
        if token.is_null() {
            return Err(glib::Error::new(
                gio::IOErrorEnum::Failed,
                "Twitch did not return a refreshed token",
            ));
        }
        Ok(Box::from_raw(token))
    }
}

// Credentials are saved only after returning to the main thread.
pub(super) unsafe fn save_refreshed_token(result: &ChannelListResult) -> Result<(), glib::Error> {
    unsafe {
        let Some(token) = &result.refreshed_token else {
            return Ok(());
        };
        if result.settings.is_null() {
            return Ok(());
        }
        app_settings_set_twitch_auth_tokens(
            result.settings,
            token.access_token.as_ptr(),
            token.refresh_token.as_ptr(),
            token_expires_at_from_expires_in(token.expires_in),
        );
        let mut error: *mut GError = ptr::null_mut();
        if app_settings_save(result.settings, &mut error) != 0 {
            return Ok(());
        }
        let err: glib::Error = from_glib_full(error);
        Err(glib::Error::with_domain(
            err.domain(),
            err.code(),
            &format!("Twitch token refreshed, but saving failed: {err}"),
        ))
    }
}

unsafe fn fetch_followed(
    token: &CString,
    cancel: *mut GCancellable,
) -> Result<Vec<TwitchFollowedChannel>, glib::Error> {
    unsafe {
        let mut error = ptr::null_mut();
        let channels = twitch_stream_info_fetch_followed_channels(
            TWITCH_AUTH_CLIENT_ID.as_ptr(),
            token.as_ptr(),
            cancel,
            &mut error,
        );
        if !error.is_null() {
            return Err(from_glib_full(error));
        }
        channels.ok_or_else(|| {
            glib::Error::new(
                gio::IOErrorEnum::Failed,
                "Followed channels could not be loaded",
            )
        })
    }
}

unsafe fn fetch_channel_list(
    data: &FetchChannelListData,
    cancel: *mut GCancellable,
) -> ChannelListResult {
    unsafe {
        let mut result = ChannelListResult {
            settings: data.settings,
            channels: unique_channels(data.manual_channels.clone()),
            refreshed_token: None,
            error: None,
        };
        if data.oauth_token.is_empty() && data.refresh_token.is_empty() {
            return result;
        }
        let followed = if let Some(cached) = fresh_followed_cache() {
            Ok(cached)
        } else {
            let mut token = data.oauth_token.clone();
            let fetch = (|| {
                if token.is_empty() || token_needs_refresh(data.oauth_expires_at) {
                    let refreshed = refresh_access_token(data, cancel)?;
                    token = refreshed.access_token.clone();
                    result.refreshed_token = Some(refreshed);
                }
                let mut followed = fetch_followed(&token, cancel);
                if followed.as_ref().is_err_and(|err| {
                    err.domain() == from_glib(twitch_stream_info_error_quark()) && err.code() == 0
                }) && result.refreshed_token.is_none()
                {
                    let refreshed = refresh_access_token(data, cancel)?;
                    token = refreshed.access_token.clone();
                    result.refreshed_token = Some(refreshed);
                    followed = fetch_followed(&token, cancel);
                }
                followed
            })();
            if let Ok(channels) = &fetch {
                store_followed_cache(channels);
            }
            fetch
        };
        match followed {
            Ok(channels) => {
                result.channels = unique_channels(
                    result.channels.into_iter().chain(
                        channels
                            .into_iter()
                            .map(|channel| channel.channel.to_string_lossy().into_owned()),
                    ),
                )
            }
            Err(err) if result.channels.is_empty() || err.matches(gio::IOErrorEnum::Cancelled) => {
                result.error = Some(err)
            }
            Err(_) => {}
        }
        result
    }
}

pub(super) unsafe extern "C" fn fetch_channel_list_worker(
    task: *mut GTask,
    _source: *mut c_void,
    data: *mut c_void,
    cancel: *mut GCancellable,
) {
    unsafe {
        let result = fetch_channel_list(&*data.cast::<FetchChannelListData>(), cancel);
        g_task_return_pointer(
            task,
            Box::into_raw(Box::new(result)).cast(),
            Some(drop_box::<ChannelListResult>),
        );
    }
}

fn followed_cache() -> &'static Mutex<FollowedCache> {
    static CACHE: OnceLock<Mutex<FollowedCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(FollowedCache {
            channels: Vec::new(),
            cached_at: None,
        })
    })
}

fn fresh_followed_cache() -> Option<Vec<TwitchFollowedChannel>> {
    let cache = followed_cache()
        .lock()
        .expect("followed channel cache poisoned");
    cache
        .cached_at
        .filter(|time| time.elapsed() < Duration::from_secs(FOLLOWED_CHANNELS_CACHE_SECONDS))
        .map(|_| cache.channels.clone())
}

fn store_followed_cache(channels: &[TwitchFollowedChannel]) {
    let mut cache = followed_cache()
        .lock()
        .expect("followed channel cache poisoned");
    cache.channels = channels.to_vec();
    cache.cached_at = Some(Instant::now());
}

pub fn twitch_channel_list_invalidate_followed_cache() {
    let mut cache = followed_cache()
        .lock()
        .expect("followed channel cache poisoned");
    cache.channels.clear();
    cache.cached_at = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_merge_keeps_order_and_ignores_case_and_empty_names() {
        assert_eq!(
            unique_channels(["Foo", "", "BAR", "foo", "Baz", "bar"].map(String::from)),
            ["foo", "bar", "baz"]
        );
    }

    #[test]
    fn followed_cache_snapshot_survives_invalidation_and_expires() {
        store_followed_cache(&[TwitchFollowedChannel {
            channel: c"example".into(),
            display_name: c"Example".into(),
        }]);
        let snapshot = fresh_followed_cache().unwrap();
        twitch_channel_list_invalidate_followed_cache();
        assert!(fresh_followed_cache().is_none());
        assert_eq!(snapshot[0].channel.as_c_str(), c"example");
        store_followed_cache(&snapshot);
        followed_cache().lock().unwrap().cached_at =
            Some(Instant::now() - Duration::from_secs(FOLLOWED_CHANNELS_CACHE_SECONDS + 1));
        assert!(fresh_followed_cache().is_none());
        twitch_channel_list_invalidate_followed_cache();
    }
}
