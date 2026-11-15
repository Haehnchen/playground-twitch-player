use super::chat::{activate_tile_chat, sync_chat_for_tile};
use super::controls::show_tile_overlay;
use super::ffi::{
    G_LOG_LEVEL_DEBUG, G_SOURCE_REMOVE, GtkButton, GtkGLArea, MPV_END_FILE_REASON_EOF,
    MPV_END_FILE_REASON_ERROR, MPV_EVENT_END_FILE, MPV_EVENT_FILE_LOADED, MPV_EVENT_LOG_MESSAGE,
    MPV_EVENT_NONE, MPV_EVENT_SHUTDOWN, MPV_EVENT_START_FILE, MPV_EVENT_VIDEO_RECONFIG,
    MPV_MAINLOOP_PRIORITY, MpvEventEndFile, MpvEventLogMessage, g_atomic_int_compare_and_exchange,
    g_atomic_int_set, g_free, g_log, g_strdup, g_strdup_printf, gtk_gl_area_queue_render,
    gtk_widget_get_realized, gtk_widget_set_sensitive, gtk_widget_set_visible, mpv_wait_event,
};
use super::render::{
    clear_tile_render_context, create_mpv_render_context, queue_tile_render, start_render_warmup,
};
use super::runtime::{
    add_tile_idle, clear_pointer, disable_tile_wakeup_callback, enable_tile_wakeup_callback,
    tile_is_closing,
};
use super::stream_details::{
    hide_tile_twitch_playback_panel, set_tile_twitch_playback_loading,
    set_tile_twitch_playback_not_checked, set_tile_twitch_playback_text,
};
use super::streams::{request_tile_qualities_update, reset_tile_quality_selection};
use super::streams::{
    request_tile_title_update, reset_tile_stream_title, update_tile_channel_label,
};
use super::{FALSE, G_LOG_DOMAIN, PLAYER_STARTING_STREAM_STATUS, StreamTile, TRUE};
use crate::glib_strings::is_nonempty;
use crate::player::channel_switcher_overlay::channel_switcher_overlay_show_at;
use crate::player::session::{
    MpvHandle, player_session_drop_buffers, player_session_free, player_session_get_channel,
    player_session_get_label, player_session_get_mpv, player_session_is_playing,
    player_session_is_ready, player_session_load_stream, player_session_new,
    player_session_set_hwdec_enabled, player_session_set_ytdl_raw_options, player_session_stop,
};
use crate::player::volume::player_volume_update_mute_button;
use crate::settings::{
    AppSettingsChannel, app_settings_get_hwdec_enabled, app_settings_get_twitch_playback_auth_token,
};
use crate::twitch::channel::normalize_twitch_channel;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

pub(super) unsafe fn tile_mpv(tile: *mut StreamTile) -> *mut MpvHandle {
    unsafe { player_session_get_mpv((*tile).session) }
}

// Drain queued mpv events without waiting for new ones.
pub(super) unsafe extern "C" fn process_mpv_events(user_data: *mut c_void) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;

        g_atomic_int_set(&mut (*tile).event_queued, 0);

        let mpv = tile_mpv(tile);
        if (*(*tile).app).closing.load(Ordering::Acquire) || mpv.is_null() {
            return G_SOURCE_REMOVE;
        }

        loop {
            let event = mpv_wait_event(mpv, 0.0);
            if event.is_null() || (*event).event_id == MPV_EVENT_NONE {
                break;
            }

            match (*event).event_id {
                MPV_EVENT_START_FILE => set_tile_status(tile, c"Loading".as_ptr()),
                MPV_EVENT_FILE_LOADED => set_tile_status(tile, c"Playback running".as_ptr()),
                MPV_EVENT_END_FILE => {
                    let end = (*event).data as *mut MpvEventEndFile;
                    if !end.is_null() && (*end).reason == MPV_END_FILE_REASON_ERROR {
                        set_tile_status(tile, c"Stream could not be played".as_ptr());
                    } else if end.is_null() || (*end).reason == MPV_END_FILE_REASON_EOF {
                        set_tile_status(tile, c"Stopped".as_ptr());
                    }
                }
                MPV_EVENT_VIDEO_RECONFIG => {}
                MPV_EVENT_LOG_MESSAGE => {
                    let log = (*event).data as *mut MpvEventLogMessage;
                    if !log.is_null() && !(*log).prefix.is_null() && !(*log).text.is_null() {
                        g_log(
                            G_LOG_DOMAIN.as_ptr() as *const c_char,
                            G_LOG_LEVEL_DEBUG,
                            c"mpv[%s][%u]: %s".as_ptr(),
                            (*log).prefix,
                            (*tile).index,
                            (*log).text,
                        );
                    }
                }
                MPV_EVENT_SHUTDOWN => return G_SOURCE_REMOVE,
                _ => {}
            }
        }

        G_SOURCE_REMOVE
    }
}

pub(super) unsafe extern "C" fn on_mpv_wakeup(ctx: *mut c_void) {
    unsafe {
        let tile = ctx as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        if g_atomic_int_compare_and_exchange(&mut (*tile).event_queued, 0, 1) != 0 {
            add_tile_idle(MPV_MAINLOOP_PRIORITY, Some(process_mpv_events), tile);
        }
    }
}

pub(super) unsafe fn dup_twitch_channel_name(value: *const c_char) -> *mut c_char {
    unsafe {
        if !is_nonempty(value) {
            return ptr::null_mut();
        }

        let Some(mut channel) = normalize_twitch_channel(CStr::from_ptr(value).to_bytes()) else {
            return ptr::null_mut();
        };

        channel.push(0);
        g_strdup(channel.as_ptr() as *const c_char)
    }
}

pub(super) unsafe fn target_to_label(target: *const c_char, channel: *const c_char) -> *mut c_char {
    unsafe {
        if is_nonempty(channel) {
            return g_strdup(channel);
        }

        if is_nonempty(target) {
            g_strdup(target)
        } else {
            ptr::null_mut()
        }
    }
}

pub(super) unsafe fn set_tile_status(_tile: *mut StreamTile, _message: *const c_char) {}

pub(super) unsafe fn sync_tile_from_session(tile: *mut StreamTile) {
    unsafe {
        if player_session_is_playing((*tile).session) == 0 {
            return;
        }

        let label = player_session_get_label((*tile).session);
        let channel = player_session_get_channel((*tile).session);

        g_free((*tile).label as *mut c_void);
        g_free((*tile).channel as *mut c_void);
        (*tile).channel = if is_nonempty(channel) {
            g_strdup(channel)
        } else {
            ptr::null_mut()
        };
        (*tile).label = g_strdup(if is_nonempty(label) {
            label
        } else {
            (*tile).channel
        });
    }
}

pub(super) unsafe fn update_tile_empty_state(tile: *mut StreamTile) {
    unsafe {
        let has_stream = is_nonempty((*tile).channel) as c_int;

        if !(*tile).empty_label.is_null() {
            gtk_widget_set_visible((*tile).empty_label, (has_stream == 0) as c_int);
        }
        if !(*tile).close_button.is_null() {
            gtk_widget_set_sensitive((*tile).close_button, TRUE);
        }
        if !(*tile).stream_info_button.is_null() {
            gtk_widget_set_sensitive((*tile).stream_info_button, TRUE);
        }
        if !(*tile).mute_button.is_null() {
            gtk_widget_set_sensitive((*tile).mute_button, TRUE);
            player_volume_update_mute_button((*tile).mute_button, (*tile).session);
        }
        if !(*tile).volume_scale.is_null() {
            gtk_widget_set_sensitive(
                (*tile).volume_scale,
                (has_stream != 0 && player_session_is_ready((*tile).session) != 0) as c_int,
            );
        }
        if !(*tile).chat_toggle_button.is_null() {
            gtk_widget_set_sensitive((*tile).chat_toggle_button, has_stream);
        }
        if !(*tile).channel_refresh_button.is_null() {
            gtk_widget_set_visible((*tile).channel_refresh_button, has_stream);
        }

        update_tile_channel_label(tile);
    }
}

pub(super) unsafe fn load_tile_stream_url(tile: *mut StreamTile) {
    unsafe {
        let playback_token = app_settings_get_twitch_playback_auth_token((*(*tile).app).settings);
        if is_nonempty(playback_token) {
            let mut raw_options = b"add-headers=Authorization:OAuth ".to_vec();
            raw_options.extend_from_slice(CStr::from_ptr(playback_token).to_bytes());
            raw_options.push(0);
            player_session_set_ytdl_raw_options(
                (*tile).session,
                raw_options.as_ptr() as *const c_char,
            );
        } else {
            player_session_set_ytdl_raw_options((*tile).session, ptr::null());
        }

        let url = g_strdup_printf(c"https://www.twitch.tv/%s".as_ptr(), (*tile).channel);
        player_session_load_stream((*tile).session, url, (*tile).label, (*tile).channel);
        g_free(url as *mut c_void);
    }
}

pub(super) unsafe fn load_tile_stream(tile: *mut StreamTile) {
    unsafe {
        if player_session_is_ready((*tile).session) == 0 || !is_nonempty((*tile).channel) {
            return;
        }

        reset_tile_quality_selection(tile);
        set_tile_status(tile, PLAYER_STARTING_STREAM_STATUS);
        let token_configured = is_nonempty(app_settings_get_twitch_playback_auth_token(
            (*(*tile).app).settings,
        ));
        if token_configured {
            set_tile_twitch_playback_loading(tile);
        } else {
            set_tile_twitch_playback_text(
            tile,
            c"Connection: Anonymous\nUser ID: -\nSubscription claim: -\nTurbo: -\nShow ads: -\nHide ads: -\nServer-side ads: -\nAd-free playback: Not checked".as_ptr(),
        );
        }
        load_tile_stream_url(tile);
        if token_configured {
            request_tile_qualities_update(tile, TRUE);
        }
        update_tile_empty_state(tile);
        sync_chat_for_tile(tile);
        request_tile_title_update(tile, TRUE);
    }
}

pub(super) unsafe fn reset_owned_tile_session(tile: *mut StreamTile) {
    unsafe {
        clear_tile_render_context(tile);
        disable_tile_wakeup_callback(tile);
        if (*tile).owns_session != 0 {
            player_session_free((*tile).session);
            (*tile).session = player_session_new();
            player_session_set_hwdec_enabled(
                (*tile).session,
                app_settings_get_hwdec_enabled((*(*tile).app).settings),
            );
        } else {
            player_session_stop((*tile).session);
        }
    }
}

pub(super) unsafe fn stop_tile_stream(tile: *mut StreamTile) {
    unsafe {
        if (*tile).chat_visible != 0 {
            activate_tile_chat(tile, FALSE);
        }

        reset_owned_tile_session(tile);
        clear_pointer(&mut (*tile).label);
        clear_pointer(&mut (*tile).channel);
        reset_tile_quality_selection(tile);
        hide_tile_twitch_playback_panel(tile);
        set_tile_twitch_playback_not_checked(tile);
        reset_tile_stream_title(tile);
        update_tile_empty_state(tile);
        queue_tile_render(tile);
    }
}

pub(super) unsafe fn ensure_tile_session(tile: *mut StreamTile) -> c_int {
    unsafe {
        if (*tile).session.is_null() {
            (*tile).session = player_session_new();
            (*tile).owns_session = TRUE;
        }

        if player_session_is_ready((*tile).session) == 0 {
            update_tile_empty_state(tile);
            return FALSE;
        }

        player_session_set_hwdec_enabled(
            (*tile).session,
            app_settings_get_hwdec_enabled((*(*tile).app).settings),
        );
        enable_tile_wakeup_callback(tile);
        if !(*tile).gl_area.is_null()
            && gtk_widget_get_realized((*tile).gl_area) != 0
            && create_mpv_render_context(tile) == 0
        {
            update_tile_empty_state(tile);
            return FALSE;
        }

        update_tile_empty_state(tile);
        TRUE
    }
}

pub(super) unsafe fn set_tile_channel(tile: *mut StreamTile, channel: *const AppSettingsChannel) {
    unsafe {
        if channel.is_null() || !is_nonempty((*channel).channel) {
            return;
        }

        g_free((*tile).label as *mut c_void);
        g_free((*tile).channel as *mut c_void);
        (*tile).label = g_strdup((*channel).label);
        (*tile).channel = g_strdup((*channel).channel);
        reset_tile_quality_selection(tile);
        reset_tile_stream_title(tile);

        if ensure_tile_session(tile) == 0 {
            return;
        }

        load_tile_stream(tile);
    }
}

pub(super) unsafe extern "C" fn activate_tile_context_channel(
    channel: *const AppSettingsChannel,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        set_tile_channel(tile, channel);
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_tile_close_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        stop_tile_stream(tile);
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_empty_tile_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        channel_switcher_overlay_show_at((*tile).channel_switcher, 0.0, 0.0);
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_channel_refresh_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        if player_session_is_playing((*tile).session) == 0 {
            return;
        }

        player_session_drop_buffers((*tile).session);
        start_render_warmup(tile);
        if !(*tile).gl_area.is_null() {
            gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
        }
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_channel_button_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        channel_switcher_overlay_show_at((*tile).channel_switcher, 0.0, 0.0);
        show_tile_overlay(tile);
    }
}

pub(super) unsafe fn tile_has_stream(tile: *mut StreamTile) -> bool {
    unsafe { is_nonempty((*tile).channel) }
}

pub(super) unsafe fn tile_has_visible_stream(tile: *mut StreamTile) -> bool {
    unsafe { tile_has_stream(tile) || player_session_is_playing((*tile).session) != 0 }
}
