use crate::glib_strings::dup_bytes;
use gtk4::glib;
use libmpv2::Mpv;
use libmpv2_sys as mpv_sys;
use std::{
    ffi::{CStr, CString, c_char, c_double, c_int, c_void},
    ptr,
    rc::Rc,
};

pub struct PlayerSession {
    mpv: Option<Rc<Mpv>>,
    label: Option<CString>,
    channel: Option<CString>,
    url: Option<CString>,
    volume: c_double,
    muted: c_int,
    playing: c_int,
}

impl Drop for PlayerSession {
    fn drop(&mut self) {
        if let Some(mpv) = &self.mpv {
            unsafe {
                mpv_sys::mpv_set_wakeup_callback(mpv.ctx.as_ptr(), None, ptr::null_mut());
            }
        }
    }
}

pub fn player_session_new() -> *mut PlayerSession {
    let mpv = match init_mpv() {
        Ok(mpv) => Some(Rc::new(mpv)),
        Err(err) => {
            report(Err(err), "mpv init");
            None
        }
    };
    Box::into_raw(Box::new(PlayerSession {
        mpv,
        label: None,
        channel: None,
        url: None,
        volume: 80.0,
        muted: 0,
        playing: 0,
    }))
}

/// # Safety
/// - Release an owned session once; callbacks must already be detached.
pub unsafe fn player_session_free(session: *mut PlayerSession) {
    if !session.is_null() {
        unsafe {
            drop(Box::from_raw(session));
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_is_ready(session: *mut PlayerSession) -> c_int {
    unsafe {
        session
            .as_ref()
            .is_some_and(|session| session.mpv.is_some()) as c_int
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_is_playing(session: *mut PlayerSession) -> c_int {
    unsafe {
        session.as_ref().is_some_and(|session| {
            session.mpv.is_some()
                && session.playing != 0
                && session.url.as_ref().is_some_and(|url| !url.is_empty())
        }) as c_int
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access on the GTK main thread.
pub unsafe fn player_session_get_mpv(session: *mut PlayerSession) -> Option<Rc<Mpv>> {
    unsafe { session.as_ref().and_then(|session| session.mpv.clone()) }
}

/// # Safety
/// - `session`: null or live instance; string borrowed until the next stream change.
pub unsafe fn player_session_get_label(session: *mut PlayerSession) -> *const c_char {
    unsafe {
        session
            .as_ref()
            .and_then(|session| session.label.as_ref())
            .map_or(ptr::null(), |value| value.as_ptr())
    }
}

/// # Safety
/// - `session`: null or live instance; string borrowed until the next stream change.
pub unsafe fn player_session_get_channel(session: *mut PlayerSession) -> *const c_char {
    unsafe {
        session
            .as_ref()
            .and_then(|session| session.channel.as_ref())
            .map_or(ptr::null(), |value| value.as_ptr())
    }
}

/// # Safety
/// - `session`: null or live instance; string borrowed until the next stream change.
pub unsafe fn player_session_get_url(session: *mut PlayerSession) -> *const c_char {
    unsafe {
        session
            .as_ref()
            .and_then(|session| session.url.as_ref())
            .map_or(ptr::null(), |value| value.as_ptr())
    }
}

/// # Safety
/// - `session`: null or live instance; result owned, release with `g_free`.
pub unsafe fn player_session_dup_url(session: *mut PlayerSession) -> *mut c_char {
    unsafe {
        let url = player_session_get_url(session);
        if url.is_null() || *url == 0 {
            ptr::null_mut()
        } else {
            dup_bytes(CStr::from_ptr(url).to_bytes())
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_get_volume(session: *mut PlayerSession) -> c_double {
    unsafe { session.as_ref().map_or(80.0, |session| session.volume) }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_set_volume(session: *mut PlayerSession, volume: c_double) {
    unsafe {
        if let Some(mpv) = player_session_get_mpv(session) {
            (*session).volume = volume;
            report(mpv.set_property("volume", volume), "set volume");
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_get_muted(session: *mut PlayerSession) -> c_int {
    unsafe { session.as_ref().map_or(0, |session| session.muted) }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_set_muted(session: *mut PlayerSession, muted: c_int) {
    unsafe {
        if let Some(mpv) = player_session_get_mpv(session) {
            (*session).muted = muted;
            report(
                mpv.set_property("mute", if muted != 0 { "yes" } else { "no" }),
                "set mute",
            );
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_toggle_muted(session: *mut PlayerSession) {
    unsafe {
        player_session_set_muted(session, (player_session_get_muted(session) == 0).into());
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_set_hwdec_enabled(session: *mut PlayerSession, enabled: c_int) {
    unsafe {
        if let Some(mpv) = player_session_get_mpv(session) {
            report(
                mpv.set_property("hwdec", if enabled != 0 { "auto-safe" } else { "no" }),
                "set hwdec",
            );
        }
    }
}

/// # Safety
/// - Session live or null; non-null options readable and NUL-terminated for this call.
pub unsafe fn player_session_set_ytdl_raw_options(
    session: *mut PlayerSession,
    options: *const c_char,
) {
    unsafe {
        if let Some(mpv) = player_session_get_mpv(session) {
            let options = if options.is_null() {
                "".into()
            } else {
                CStr::from_ptr(options).to_string_lossy()
            };
            report(
                mpv.set_property("ytdl-raw-options", options.as_ref()),
                "set yt-dlp options",
            );
        }
    }
}

/// # Safety
/// - Session live or null; callback data stays valid until callbacks are disabled.
/// - Callback callable from mpv threads, declared C ABI, no unwind.
pub unsafe fn player_session_set_wakeup_callback(
    session: *mut PlayerSession,
    callback: Option<unsafe extern "C" fn(*mut c_void)>,
    data: *mut c_void,
) {
    unsafe {
        if let Some(mpv) = player_session_get_mpv(session) {
            mpv_sys::mpv_set_wakeup_callback(mpv.ctx.as_ptr(), callback, data);
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_toggle_stream_info(session: *mut PlayerSession) {
    unsafe {
        if let Some(mpv) = player_session_get_mpv(session)
            && mpv
                .command("script-binding", &["stats/display-stats-toggle"])
                .is_err()
        {
            report(mpv.command("keypress", &["i"]), "toggle stream info");
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
        if let Some(mpv) = player_session_get_mpv(session) {
            // Reattach video when a layout change creates a new GLArea.
            report(mpv.set_property("vid", "no"), "disable video");
            report(mpv.set_property("vid", "auto"), "enable video");
        }
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_drop_buffers(session: *mut PlayerSession) {
    unsafe {
        if player_session_is_playing(session) != 0
            && let Some(mpv) = player_session_get_mpv(session)
        {
            report(command_async(&mpv, c"drop-buffers", &[]), "drop buffers");
        }
    }
}

/// # Safety
/// - Session live or null; non-null strings readable and NUL-terminated for this call.
pub unsafe fn player_session_load_stream(
    session: *mut PlayerSession,
    url: *const c_char,
    label: *const c_char,
    channel: *const c_char,
) {
    unsafe {
        let Some(mpv) = player_session_get_mpv(session) else {
            return;
        };
        if url.is_null() || *url == 0 {
            return;
        }
        let url = CStr::from_ptr(url).to_owned();
        let label = (!label.is_null()).then(|| CStr::from_ptr(label).to_owned());
        let channel = (!channel.is_null()).then(|| CStr::from_ptr(channel).to_owned());
        report(
            command_async(&mpv, c"loadfile", &[url.as_c_str(), c"replace"]),
            "loadfile",
        );
        (*session).url = Some(url);
        (*session).label = label;
        (*session).channel = channel;
        (*session).playing = 1;
    }
}

/// # Safety
/// - `session`: null or live instance; serialize access.
pub unsafe fn player_session_stop(session: *mut PlayerSession) {
    unsafe {
        if let Some(mpv) = player_session_get_mpv(session) {
            report(command_async(&mpv, c"stop", &[]), "stop");
            (*session).label = None;
            (*session).channel = None;
            (*session).url = None;
            (*session).playing = 0;
        }
    }
}

// libmpv2 exposes synchronous commands; stream changes must remain queued.
fn command_async(mpv: &Mpv, name: &CStr, args: &[&CStr]) -> libmpv2::Result<()> {
    let mut args: Vec<_> = std::iter::once(name)
        .chain(args.iter().copied())
        .map(CStr::as_ptr)
        .collect();
    args.push(ptr::null());
    let status = unsafe { mpv_sys::mpv_command_async(mpv.ctx.as_ptr(), 0, args.as_mut_ptr()) };
    if status < 0 {
        Err(libmpv2::Error::Raw(status))
    } else {
        Ok(())
    }
}

pub(super) fn report(result: libmpv2::Result<()>, action: &str) {
    if let Err(error) = result {
        match error {
            libmpv2::Error::Raw(code) => {
                let message =
                    unsafe { CStr::from_ptr(mpv_sys::mpv_error_string(code)) }.to_string_lossy();
                glib::g_warning!("twitch-player", "{action}: {message}");
            }
            error => glib::g_warning!("twitch-player", "{action}: {error}"),
        }
    }
}

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

const LC_NUMERIC: c_int = 1;

fn init_mpv() -> libmpv2::Result<Mpv> {
    if unsafe { setlocale(LC_NUMERIC, c"C".as_ptr()) }.is_null() {
        glib::g_warning!("twitch-player", "Could not set LC_NUMERIC to C");
    }
    Mpv::with_initializer(|init| {
        for (name, value) in [
            ("terminal", "no"),
            ("config", "no"),
            ("vo", "libmpv"),
            ("ytdl", "yes"),
            ("hwdec", "auto-safe"),
            ("cache", "yes"),
            ("cache-pause-initial", "yes"),
            ("cache-pause-wait", "3"),
            ("cache-secs", "12"),
            ("demuxer-max-bytes", "32MiB"),
            ("demuxer-max-back-bytes", "4MiB"),
            ("demuxer-donate-buffer", "no"),
            ("demuxer-readahead-secs", "12"),
            ("stream-buffer-size", "1MiB"),
            ("video-sync", "display-resample"),
            ("video-sync-max-audio-change", "0.05"),
            ("interpolation", "yes"),
            ("volume", "80"),
        ] {
            report(init.set_option(name, value), name);
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use libmpv2::events::Event;
    use std::time::{Duration, Instant};

    #[test]
    fn session_updates_mpv_properties_and_keeps_handle_alive_for_rendering() {
        unsafe {
            let session = player_session_new();
            let mpv = player_session_get_mpv(session).expect("libmpv initialization");
            assert_eq!(mpv.get_property::<f64>("volume").unwrap(), 80.0);
            player_session_set_volume(session, 42.0);
            player_session_set_muted(session, 1);
            player_session_set_hwdec_enabled(session, 0);
            assert_eq!(mpv.get_property::<f64>("volume").unwrap(), 42.0);
            assert_eq!(mpv.get_property::<String>("mute").unwrap(), "yes");
            assert_eq!(mpv.get_property::<String>("hwdec").unwrap(), "no");
            player_session_free(session);
            assert_eq!(mpv.get_property::<f64>("volume").unwrap(), 42.0);
        }
    }

    #[test]
    fn queued_commands_report_completion_through_typed_events() {
        let mpv = init_mpv().unwrap();
        command_async(&mpv, c"stop", &[]).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Some(Ok(Event::CommandReply(0))) = mpv.wait_event(0.1) {
                return;
            }
        }
        panic!("mpv did not acknowledge the queued command");
    }

    #[test]
    fn stream_strings_are_copied_before_replacing_previous_values() {
        unsafe {
            let session = player_session_new();
            let mpv = player_session_get_mpv(session).unwrap();
            mpv.set_property("audio", "no").unwrap();
            player_session_load_stream(
                session,
                c"/tmp/twitch-player-missing-test-video".as_ptr(),
                c"Example".as_ptr(),
                c"example".as_ptr(),
            );
            let url = player_session_get_url(session);
            let label = player_session_get_label(session);
            let channel = player_session_get_channel(session);
            player_session_load_stream(session, url, label, channel);
            assert_eq!(
                CStr::from_ptr(player_session_get_label(session)),
                c"Example"
            );
            assert_eq!(
                CStr::from_ptr(player_session_get_channel(session)),
                c"example"
            );
            player_session_stop(session);
            assert!(player_session_get_url(session).is_null());
            assert_eq!(player_session_is_playing(session), 0);
            player_session_free(session);
        }
    }
}
