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
    if status < 0 {
        g_log(
            b"twitch-player-session\0".as_ptr() as *const c_char,
            G_LOG_LEVEL_WARNING,
            b"%s: %s\0".as_ptr() as *const c_char,
            action,
            mpv_error_string(status),
        );
    }
}

unsafe fn set_mpv_option(
    session: *mut PlayerSession,
    name: *const c_char,
    value: *const c_char,
    action: *const c_char,
) {
    check_mpv(mpv_set_option_string((*session).mpv, name, value), action);
}

unsafe fn init_mpv(session: *mut PlayerSession) -> c_int {
    if setlocale(LC_NUMERIC, b"C\0".as_ptr() as *const c_char).is_null() {
        g_log(
            b"twitch-player-session\0".as_ptr() as *const c_char,
            G_LOG_LEVEL_WARNING,
            b"LC_NUMERIC could not be set to C; libmpv may refuse to start\0".as_ptr()
                as *const c_char,
        );
    }

    (*session).mpv = mpv_create();
    if (*session).mpv.is_null() {
        g_log(
            b"twitch-player-session\0".as_ptr() as *const c_char,
            G_LOG_LEVEL_WARNING,
            b"mpv_create returned NULL\0".as_ptr() as *const c_char,
        );
        return 0;
    }

    set_mpv_option(
        session,
        b"terminal\0".as_ptr() as *const c_char,
        b"no\0".as_ptr() as *const c_char,
        b"set terminal\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"config\0".as_ptr() as *const c_char,
        b"no\0".as_ptr() as *const c_char,
        b"set config\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"vo\0".as_ptr() as *const c_char,
        b"libmpv\0".as_ptr() as *const c_char,
        b"set vo\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"ytdl\0".as_ptr() as *const c_char,
        b"yes\0".as_ptr() as *const c_char,
        b"set ytdl\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"hwdec\0".as_ptr() as *const c_char,
        b"auto-safe\0".as_ptr() as *const c_char,
        b"set hwdec\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"cache\0".as_ptr() as *const c_char,
        b"yes\0".as_ptr() as *const c_char,
        b"set cache\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"cache-pause-initial\0".as_ptr() as *const c_char,
        b"yes\0".as_ptr() as *const c_char,
        b"set cache pause initial\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"cache-pause-wait\0".as_ptr() as *const c_char,
        b"3\0".as_ptr() as *const c_char,
        b"set cache pause wait\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"cache-secs\0".as_ptr() as *const c_char,
        b"12\0".as_ptr() as *const c_char,
        b"set cache seconds\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"demuxer-max-bytes\0".as_ptr() as *const c_char,
        b"96MiB\0".as_ptr() as *const c_char,
        b"set demuxer max bytes\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"demuxer-readahead-secs\0".as_ptr() as *const c_char,
        b"12\0".as_ptr() as *const c_char,
        b"set demuxer readahead\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"stream-buffer-size\0".as_ptr() as *const c_char,
        b"8MiB\0".as_ptr() as *const c_char,
        b"set stream buffer size\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"video-sync\0".as_ptr() as *const c_char,
        b"display-resample\0".as_ptr() as *const c_char,
        b"set video sync\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"video-sync-max-audio-change\0".as_ptr() as *const c_char,
        b"0.05\0".as_ptr() as *const c_char,
        b"set video sync audio change\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"interpolation\0".as_ptr() as *const c_char,
        b"yes\0".as_ptr() as *const c_char,
        b"set interpolation\0".as_ptr() as *const c_char,
    );
    set_mpv_option(
        session,
        b"volume\0".as_ptr() as *const c_char,
        b"80\0".as_ptr() as *const c_char,
        b"set volume\0".as_ptr() as *const c_char,
    );

    let status = mpv_initialize((*session).mpv);
    if status < 0 {
        g_log(
            b"twitch-player-session\0".as_ptr() as *const c_char,
            G_LOG_LEVEL_WARNING,
            b"mpv init: %s\0".as_ptr() as *const c_char,
            mpv_error_string(status),
        );
        mpv_terminate_destroy((*session).mpv);
        (*session).mpv = ptr::null_mut();
        return 0;
    }

    1
}

unsafe fn clear_string(value: &mut *mut c_char) {
    if !value.is_null() {
        let old_value = *value;
        *value = ptr::null_mut();
        g_free(old_value as *mut c_void);
    }
}

pub unsafe fn player_session_new() -> *mut PlayerSession {
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

pub unsafe fn player_session_free(session: *mut PlayerSession) {
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

pub unsafe fn player_session_is_ready(session: *mut PlayerSession) -> c_int {
    (!session.is_null() && !(*session).mpv.is_null()) as c_int
}

pub unsafe fn player_session_is_playing(session: *mut PlayerSession) -> c_int {
    (player_session_is_ready(session) != 0
        && (*session).playing != 0
        && !(*session).url.is_null()
        && *(*session).url != 0) as c_int
}

pub unsafe fn player_session_get_mpv(session: *mut PlayerSession) -> *mut MpvHandle {
    if session.is_null() {
        return ptr::null_mut();
    }

    (*session).mpv
}

pub unsafe fn player_session_get_label(session: *mut PlayerSession) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }

    (*session).label
}

pub unsafe fn player_session_get_channel(session: *mut PlayerSession) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }

    (*session).channel
}

pub unsafe fn player_session_get_url(session: *mut PlayerSession) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }

    (*session).url
}

pub unsafe fn player_session_dup_url(session: *mut PlayerSession) -> *mut c_char {
    let url = player_session_get_url(session);
    if !url.is_null() && *url != 0 {
        return g_strdup(url);
    }

    ptr::null_mut()
}

pub unsafe fn player_session_get_volume(session: *mut PlayerSession) -> c_double {
    if session.is_null() {
        return 80.0;
    }

    (*session).volume
}

pub unsafe fn player_session_set_volume(session: *mut PlayerSession, volume: c_double) {
    if player_session_is_ready(session) == 0 {
        return;
    }

    (*session).volume = volume;
    let mut value = volume;
    check_mpv(
        mpv_set_property(
            (*session).mpv,
            b"volume\0".as_ptr() as *const c_char,
            MPV_FORMAT_DOUBLE,
            &mut value as *mut c_double as *mut c_void,
        ),
        b"set volume\0".as_ptr() as *const c_char,
    );
}

pub unsafe fn player_session_get_muted(session: *mut PlayerSession) -> c_int {
    if session.is_null() {
        return 0;
    }

    (*session).muted
}

pub unsafe fn player_session_set_muted(session: *mut PlayerSession, muted: c_int) {
    if player_session_is_ready(session) == 0 {
        return;
    }

    (*session).muted = muted;
    check_mpv(
        mpv_set_property_string(
            (*session).mpv,
            b"mute\0".as_ptr() as *const c_char,
            if muted != 0 {
                b"yes\0".as_ptr() as *const c_char
            } else {
                b"no\0".as_ptr() as *const c_char
            },
        ),
        if muted != 0 {
            b"mute\0".as_ptr() as *const c_char
        } else {
            b"unmute\0".as_ptr() as *const c_char
        },
    );
}

pub unsafe fn player_session_toggle_muted(session: *mut PlayerSession) {
    player_session_set_muted(session, (player_session_get_muted(session) == 0) as c_int);
}

pub unsafe fn player_session_set_hwdec_enabled(session: *mut PlayerSession, enabled: c_int) {
    if player_session_is_ready(session) == 0 {
        return;
    }

    check_mpv(
        mpv_set_property_string(
            (*session).mpv,
            b"hwdec\0".as_ptr() as *const c_char,
            if enabled != 0 {
                b"auto-safe\0".as_ptr() as *const c_char
            } else {
                b"no\0".as_ptr() as *const c_char
            },
        ),
        b"set hwdec\0".as_ptr() as *const c_char,
    );
}

pub unsafe fn player_session_set_wakeup_callback(
    session: *mut PlayerSession,
    callback: Option<unsafe extern "C" fn(*mut c_void)>,
    data: *mut c_void,
) {
    if player_session_is_ready(session) != 0 {
        mpv_set_wakeup_callback((*session).mpv, callback, data);
    }
}

pub unsafe fn player_session_toggle_stream_info(session: *mut PlayerSession) {
    if player_session_is_ready(session) == 0 {
        return;
    }

    let stats_cmd = [
        b"script-binding\0".as_ptr() as *const c_char,
        b"stats/display-stats-toggle\0".as_ptr() as *const c_char,
        ptr::null(),
    ];
    let status = mpv_command((*session).mpv, stats_cmd.as_ptr());
    if status < 0 {
        let keypress_cmd = [
            b"keypress\0".as_ptr() as *const c_char,
            b"i\0".as_ptr() as *const c_char,
            ptr::null(),
        ];
        check_mpv(
            mpv_command((*session).mpv, keypress_cmd.as_ptr()),
            b"toggle stream info\0".as_ptr() as *const c_char,
        );
    }
}

pub unsafe fn player_session_reenable_video(session: *mut PlayerSession) {
    if player_session_is_playing(session) == 0 {
        return;
    }

    check_mpv(
        mpv_set_property_string(
            (*session).mpv,
            b"vid\0".as_ptr() as *const c_char,
            b"no\0".as_ptr() as *const c_char,
        ),
        b"disable video\0".as_ptr() as *const c_char,
    );
    check_mpv(
        mpv_set_property_string(
            (*session).mpv,
            b"vid\0".as_ptr() as *const c_char,
            b"auto\0".as_ptr() as *const c_char,
        ),
        b"enable video\0".as_ptr() as *const c_char,
    );
}

pub unsafe fn player_session_load_stream(
    session: *mut PlayerSession,
    url: *const c_char,
    label: *const c_char,
    channel: *const c_char,
) {
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
        b"loadfile\0".as_ptr() as *const c_char,
        (*session).url as *const c_char,
        b"replace\0".as_ptr() as *const c_char,
        ptr::null(),
    ];
    check_mpv(
        mpv_command_async((*session).mpv, 0, cmd.as_ptr()),
        b"loadfile\0".as_ptr() as *const c_char,
    );
}

pub unsafe fn player_session_stop(session: *mut PlayerSession) {
    if player_session_is_ready(session) == 0 {
        return;
    }

    let cmd = [b"stop\0".as_ptr() as *const c_char, ptr::null()];
    check_mpv(
        mpv_command_async((*session).mpv, 0, cmd.as_ptr()),
        b"stop\0".as_ptr() as *const c_char,
    );

    clear_string(&mut (*session).label);
    clear_string(&mut (*session).channel);
    clear_string(&mut (*session).url);
    (*session).playing = 0;
}
