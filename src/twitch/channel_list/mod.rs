mod fetch;
mod ffi;

pub use self::fetch::twitch_channel_list_invalidate_followed_cache;
use self::fetch::{collect_settings_channels, fetch_channel_list_worker, save_refreshed_token};
pub use self::ffi::GTask;
use self::ffi::{
    g_object_unref, g_task_is_valid, g_task_new, g_task_propagate_pointer, g_task_run_in_thread,
    g_task_set_task_data,
};
use crate::settings::{
    AppSettings, app_settings_free, app_settings_get_twitch_oauth_expires_at,
    app_settings_get_twitch_oauth_token, app_settings_get_twitch_refresh_token, app_settings_ref,
};
use crate::twitch::auth::TwitchAuthToken;
use crate::twitch::stream_info::{GAsyncResult, GCancellable, GError, TwitchFollowedChannel};
use gtk4::glib::{self, translate::*};
use std::ffi::{CStr, CString, c_void};
use std::ptr;

struct FetchChannelListData {
    settings: *mut AppSettings,
    manual_channels: Vec<String>,
    oauth_token: CString,
    refresh_token: CString,
    oauth_expires_at: i64,
}

impl Drop for FetchChannelListData {
    fn drop(&mut self) {
        unsafe {
            app_settings_free(self.settings);
        }
    }
}

struct ChannelListResult {
    settings: *mut AppSettings,
    channels: Vec<String>,
    refreshed_token: Option<Box<TwitchAuthToken>>,
    error: Option<glib::Error>,
}

struct FollowedCache {
    channels: Vec<TwitchFollowedChannel>,
    cached_at: Option<std::time::Instant>,
}

type GAsyncReadyCallback =
    Option<unsafe extern "C" fn(*mut c_void, *mut GAsyncResult, *mut c_void)>;
type GDestroyNotify = unsafe extern "C" fn(*mut c_void);
type GTaskThreadFunc =
    unsafe extern "C" fn(*mut GTask, *mut c_void, *mut c_void, *mut GCancellable);

unsafe extern "C" fn drop_box<T>(data: *mut c_void) {
    unsafe {
        if !data.is_null() {
            drop(Box::from_raw(data.cast::<T>()));
        }
    }
}

/// # Safety
/// GTK main thread; settings and callback data remain live through completion.
/// The callback uses the GAsyncReadyCallback ABI and must not unwind.
pub unsafe fn twitch_channel_list_fetch_async(
    settings: *mut AppSettings,
    cancel: *mut GCancellable,
    callback: GAsyncReadyCallback,
    user_data: *mut c_void,
) {
    unsafe {
        let copy = |value: *const std::ffi::c_char| {
            if value.is_null() {
                CString::default()
            } else {
                CStr::from_ptr(value).to_owned()
            }
        };
        let data = FetchChannelListData {
            settings: app_settings_ref(settings),
            manual_channels: collect_settings_channels(settings),
            oauth_token: copy(app_settings_get_twitch_oauth_token(settings)),
            refresh_token: copy(app_settings_get_twitch_refresh_token(settings)),
            oauth_expires_at: app_settings_get_twitch_oauth_expires_at(settings),
        };
        let task = g_task_new(ptr::null_mut(), cancel, callback, user_data);
        g_task_set_task_data(
            task,
            Box::into_raw(Box::new(data)).cast(),
            Some(drop_box::<FetchChannelListData>),
        );
        g_task_run_in_thread(task, Some(fetch_channel_list_worker));
        g_object_unref(task.cast());
    }
}

/// # Safety
/// Consume the matching async result once on the settings-owning main context.
/// `error` is null or an initially null, writable GError slot.
pub unsafe fn twitch_channel_list_fetch_finish(
    result: *mut GAsyncResult,
    error: *mut *mut GError,
) -> Option<Vec<String>> {
    unsafe {
        if g_task_is_valid(result.cast(), ptr::null_mut()) == 0 {
            return None;
        }
        let list = g_task_propagate_pointer(result.cast(), error).cast::<ChannelListResult>();
        if list.is_null() {
            return None;
        }
        let mut list = Box::from_raw(list);
        if let Err(err) = save_refreshed_token(&list) {
            glib::ffi::g_propagate_error(error, err.into_glib_ptr());
            return None;
        }
        if let Some(err) = list.error.take() {
            glib::ffi::g_propagate_error(error, err.into_glib_ptr());
            return None;
        }
        Some(list.channels)
    }
}
