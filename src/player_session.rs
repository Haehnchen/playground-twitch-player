use crate::glib_strings::clear_string;
use std::ffi::{c_char, c_double, c_int, c_void};

use std::ptr;

const G_LOG_LEVEL_WARNING: c_int = 1 << 4;
const LC_NUMERIC: c_int = 1;
const MPV_FORMAT_DOUBLE: c_int = 5;

#[repr(C)]
pub struct MpvHandle {
    _private: [u8; 0],
}

pub struct PlayerSession {
    mpv: *mut MpvHandle,
    label: *mut c_char,
    channel: *mut c_char,
    url: *mut c_char,
    volume: c_double,
    muted: c_int,
    playing: c_int,
}

unsafe extern "C" {
    fn g_free(mem: *mut c_void);
    fn g_log(log_domain: *const c_char, log_level: c_int, format: *const c_char, ...);
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn mpv_command(ctx: *mut MpvHandle, args: *const *const c_char) -> c_int;
    fn mpv_command_async(
        ctx: *mut MpvHandle,
        reply_userdata: u64,
        args: *const *const c_char,
    ) -> c_int;
    fn mpv_create() -> *mut MpvHandle;
    fn mpv_error_string(error: c_int) -> *const c_char;
    fn mpv_initialize(ctx: *mut MpvHandle) -> c_int;
    fn mpv_set_option_string(
        ctx: *mut MpvHandle,
        name: *const c_char,
        data: *const c_char,
    ) -> c_int;
    fn mpv_set_property(
        ctx: *mut MpvHandle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    fn mpv_set_property_string(
        ctx: *mut MpvHandle,
        name: *const c_char,
        data: *const c_char,
    ) -> c_int;
    fn mpv_set_wakeup_callback(
        ctx: *mut MpvHandle,
        cb: Option<unsafe extern "C" fn(*mut c_void)>,
        data: *mut c_void,
    );
    fn mpv_terminate_destroy(ctx: *mut MpvHandle);
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

unsafe fn check_mpv(status: c_int, action: *const c_char) {
    unsafe {
        if status < 0 {
            g_log(
                c"twitch-player-session".as_ptr(),
                G_LOG_LEVEL_WARNING,
                c"%s: %s".as_ptr(),
                action,
                mpv_error_string(status),
            );
        }
    }
}

unsafe fn set_mpv_option(
    session: *mut PlayerSession,
    name: *const c_char,
    value: *const c_char,
    action: *const c_char,
) {
    unsafe {
        check_mpv(mpv_set_option_string((*session).mpv, name, value), action);
    }
}

unsafe fn init_mpv(session: *mut PlayerSession) -> c_int {
    unsafe {
        if setlocale(LC_NUMERIC, c"C".as_ptr()).is_null() {
            g_log(
                c"twitch-player-session".as_ptr(),
                G_LOG_LEVEL_WARNING,
                c"LC_NUMERIC could not be set to C; libmpv may refuse to start".as_ptr(),
            );
        }

        (*session).mpv = mpv_create();
        if (*session).mpv.is_null() {
            g_log(
                c"twitch-player-session".as_ptr(),
                G_LOG_LEVEL_WARNING,
                c"mpv_create returned NULL".as_ptr(),
            );
            return 0;
        }

        set_mpv_option(
            session,
            c"terminal".as_ptr(),
            c"no".as_ptr(),
            c"set terminal".as_ptr(),
        );
        set_mpv_option(
            session,
            c"config".as_ptr(),
            c"no".as_ptr(),
            c"set config".as_ptr(),
        );
        set_mpv_option(
            session,
            c"vo".as_ptr(),
            c"libmpv".as_ptr(),
            c"set vo".as_ptr(),
        );
        set_mpv_option(
            session,
            c"ytdl".as_ptr(),
            c"yes".as_ptr(),
            c"set ytdl".as_ptr(),
        );
        set_mpv_option(
            session,
            c"hwdec".as_ptr(),
            c"auto-safe".as_ptr(),
            c"set hwdec".as_ptr(),
        );
        set_mpv_option(
            session,
            c"cache".as_ptr(),
            c"yes".as_ptr(),
            c"set cache".as_ptr(),
        );
        set_mpv_option(
            session,
            c"cache-pause-initial".as_ptr(),
            c"yes".as_ptr(),
            c"set cache pause initial".as_ptr(),
        );
        set_mpv_option(
            session,
            c"cache-pause-wait".as_ptr(),
            c"3".as_ptr(),
            c"set cache pause wait".as_ptr(),
        );
        set_mpv_option(
            session,
            c"cache-secs".as_ptr(),
            c"12".as_ptr(),
            c"set cache seconds".as_ptr(),
        );
        set_mpv_option(
            session,
            c"demuxer-max-bytes".as_ptr(),
            c"96MiB".as_ptr(),
            c"set demuxer max bytes".as_ptr(),
        );
        set_mpv_option(
            session,
            c"demuxer-readahead-secs".as_ptr(),
            c"12".as_ptr(),
            c"set demuxer readahead".as_ptr(),
        );
        set_mpv_option(
            session,
            c"stream-buffer-size".as_ptr(),
            c"8MiB".as_ptr(),
            c"set stream buffer size".as_ptr(),
        );
        set_mpv_option(
            session,
            c"video-sync".as_ptr(),
            c"display-resample".as_ptr(),
            c"set video sync".as_ptr(),
        );
        set_mpv_option(
            session,
            c"video-sync-max-audio-change".as_ptr(),
            c"0.05".as_ptr(),
            c"set video sync audio change".as_ptr(),
        );
        set_mpv_option(
            session,
            c"interpolation".as_ptr(),
            c"yes".as_ptr(),
            c"set interpolation".as_ptr(),
        );
        set_mpv_option(
            session,
            c"volume".as_ptr(),
            c"80".as_ptr(),
            c"set volume".as_ptr(),
        );

        let status = mpv_initialize((*session).mpv);
        if status < 0 {
            g_log(
                c"twitch-player-session".as_ptr(),
                G_LOG_LEVEL_WARNING,
                c"mpv init: %s".as_ptr(),
                mpv_error_string(status),
            );
            mpv_terminate_destroy((*session).mpv);
            (*session).mpv = ptr::null_mut();
            return 0;
        }

        1
    }
}

pub fn player_session_new() -> *mut PlayerSession {
    unsafe {
        let mut session = Box::new(PlayerSession {
            mpv: ptr::null_mut(),
            label: ptr::null_mut(),
            channel: ptr::null_mut(),
            url: ptr::null_mut(),
            volume: 80.0,
            muted: 0,
            playing: 0,
        });
        init_mpv(&mut *session);
        Box::into_raw(session)
    }
}

/// # Safety
/// - `session`: null or owned instance; release once.
/// - Destroy render contexts first; no remaining handle/string users.
pub unsafe fn player_session_free(session: *mut PlayerSession) {
    unsafe {
        if session.is_null() {
            return;
        }

        if !(*session).mpv.is_null() {
            mpv_set_wakeup_callback((*session).mpv, None, ptr::null_mut());
            mpv_terminate_destroy((*session).mpv);
            (*session).mpv = ptr::null_mut();
        }

        g_free((*session).label as *mut c_void);
        g_free((*session).channel as *mut c_void);
        g_free((*session).url as *mut c_void);
        drop(Box::from_raw(session));
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_is_ready(session: *mut PlayerSession) -> c_int {
    unsafe { (!session.is_null() && !(*session).mpv.is_null()) as c_int }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_is_playing(session: *mut PlayerSession) -> c_int {
    unsafe {
        (player_session_is_ready(session) != 0
            && (*session).playing != 0
            && !(*session).url.is_null()
            && *(*session).url != 0) as c_int
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Returned mpv handle: borrowed until session destruction; do not destroy.
pub unsafe fn player_session_get_mpv(session: *mut PlayerSession) -> *mut MpvHandle {
    unsafe {
        if session.is_null() {
            return ptr::null_mut();
        }

        (*session).mpv
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Returned string: borrowed until stream change, stop, or session destruction.
pub unsafe fn player_session_get_label(session: *mut PlayerSession) -> *const c_char {
    unsafe {
        if session.is_null() {
            return ptr::null();
        }

        (*session).label
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Returned string: borrowed until stream change, stop, or session destruction.
pub unsafe fn player_session_get_channel(session: *mut PlayerSession) -> *const c_char {
    unsafe {
        if session.is_null() {
            return ptr::null();
        }

        (*session).channel
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Returned string: borrowed until stream change, stop, or session destruction.
pub unsafe fn player_session_get_url(session: *mut PlayerSession) -> *const c_char {
    unsafe {
        if session.is_null() {
            return ptr::null();
        }

        (*session).url
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Returned string: owned; release with `g_free`.
pub unsafe fn player_session_dup_url(session: *mut PlayerSession) -> *mut c_char {
    unsafe {
        let url = player_session_get_url(session);
        if !url.is_null() && *url != 0 {
            return g_strdup(url);
        }

        ptr::null_mut()
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_get_volume(session: *mut PlayerSession) -> c_double {
    unsafe {
        if session.is_null() {
            return 80.0;
        }

        (*session).volume
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_set_volume(session: *mut PlayerSession, volume: c_double) {
    unsafe {
        if player_session_is_ready(session) == 0 {
            return;
        }

        (*session).volume = volume;
        let mut value = volume;
        check_mpv(
            mpv_set_property(
                (*session).mpv,
                c"volume".as_ptr(),
                MPV_FORMAT_DOUBLE,
                &mut value as *mut c_double as *mut c_void,
            ),
            c"set volume".as_ptr(),
        );
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_get_muted(session: *mut PlayerSession) -> c_int {
    unsafe {
        if session.is_null() {
            return 0;
        }

        (*session).muted
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_set_muted(session: *mut PlayerSession, muted: c_int) {
    unsafe {
        if player_session_is_ready(session) == 0 {
            return;
        }

        (*session).muted = muted;
        check_mpv(
            mpv_set_property_string(
                (*session).mpv,
                c"mute".as_ptr(),
                if muted != 0 {
                    c"yes".as_ptr()
                } else {
                    c"no".as_ptr()
                },
            ),
            if muted != 0 {
                c"mute".as_ptr()
            } else {
                c"unmute".as_ptr()
            },
        );
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_toggle_muted(session: *mut PlayerSession) {
    unsafe {
        player_session_set_muted(session, (player_session_get_muted(session) == 0) as c_int);
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_set_hwdec_enabled(session: *mut PlayerSession, enabled: c_int) {
    unsafe {
        if player_session_is_ready(session) == 0 {
            return;
        }

        check_mpv(
            mpv_set_property_string(
                (*session).mpv,
                c"hwdec".as_ptr(),
                if enabled != 0 {
                    c"auto-safe".as_ptr()
                } else {
                    c"no".as_ptr()
                },
            ),
            c"set hwdec".as_ptr(),
        );
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
pub unsafe fn player_session_set_ytdl_raw_options(
    session: *mut PlayerSession,
    options: *const c_char,
) {
    unsafe {
        if player_session_is_ready(session) == 0 {
            return;
        }

        check_mpv(
            mpv_set_property_string(
                (*session).mpv,
                c"ytdl-raw-options".as_ptr(),
                if options.is_null() {
                    c"".as_ptr()
                } else {
                    options
                },
            ),
            c"set yt-dlp options".as_ptr(),
        );
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Callback: C ABI, no unwind, callable from mpv threads.
/// - `data`: valid until callbacks are disabled and all in-flight calls finish.
pub unsafe fn player_session_set_wakeup_callback(
    session: *mut PlayerSession,
    callback: Option<unsafe extern "C" fn(*mut c_void)>,
    data: *mut c_void,
) {
    unsafe {
        if player_session_is_ready(session) != 0 {
            mpv_set_wakeup_callback((*session).mpv, callback, data);
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_toggle_stream_info(session: *mut PlayerSession) {
    unsafe {
        if player_session_is_ready(session) == 0 {
            return;
        }

        let stats_cmd = [
            c"script-binding".as_ptr(),
            c"stats/display-stats-toggle".as_ptr(),
            ptr::null(),
        ];
        let status = mpv_command((*session).mpv, stats_cmd.as_ptr());
        if status < 0 {
            let keypress_cmd = [c"keypress".as_ptr(), c"i".as_ptr(), ptr::null()];
            check_mpv(
                mpv_command((*session).mpv, keypress_cmd.as_ptr()),
                c"toggle stream info".as_ptr(),
            );
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_reenable_video(session: *mut PlayerSession) {
    unsafe {
        if player_session_is_playing(session) == 0 {
            return;
        }

        // Rebind video after a new libmpv render context is attached to a GTK GLArea.
        check_mpv(
            mpv_set_property_string((*session).mpv, c"vid".as_ptr(), c"no".as_ptr()),
            c"disable video".as_ptr(),
        );
        check_mpv(
            mpv_set_property_string((*session).mpv, c"vid".as_ptr(), c"auto".as_ptr()),
            c"enable video".as_ptr(),
        );
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_drop_buffers(session: *mut PlayerSession) {
    unsafe {
        if player_session_is_playing(session) == 0 {
            return;
        }

        // Discard live stream buffers so audio/video can resync without reloading the URL.
        let cmd = [c"drop-buffers".as_ptr(), ptr::null()];
        check_mpv(
            mpv_command_async((*session).mpv, 0, cmd.as_ptr()),
            c"drop buffers".as_ptr(),
        );
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
/// - Non-null strings: readable, NUL-terminated UTF-8 for this call.
/// - Inputs must not alias session strings; those are freed before copying.
pub unsafe fn player_session_load_stream(
    session: *mut PlayerSession,
    url: *const c_char,
    label: *const c_char,
    channel: *const c_char,
) {
    unsafe {
        if player_session_is_ready(session) == 0 || url.is_null() || *url == 0 {
            return;
        }

        g_free((*session).label as *mut c_void);
        g_free((*session).channel as *mut c_void);
        g_free((*session).url as *mut c_void);
        (*session).label = g_strdup(label);
        (*session).channel = g_strdup(channel);
        (*session).url = g_strdup(url);
        (*session).playing = 1;

        let cmd = [
            c"loadfile".as_ptr(),
            (*session).url as *const c_char,
            c"replace".as_ptr(),
            ptr::null(),
        ];
        check_mpv(
            mpv_command_async((*session).mpv, 0, cmd.as_ptr()),
            c"loadfile".as_ptr(),
        );
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_stop(session: *mut PlayerSession) {
    unsafe {
        if player_session_is_ready(session) == 0 {
            return;
        }

        let cmd = [c"stop".as_ptr(), ptr::null()];
        check_mpv(
            mpv_command_async((*session).mpv, 0, cmd.as_ptr()),
            c"stop".as_ptr(),
        );

        clear_string(&mut (*session).label);
        clear_string(&mut (*session).channel);
        clear_string(&mut (*session).url);
        (*session).playing = 0;
    }
}
