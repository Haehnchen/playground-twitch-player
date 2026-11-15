use super::chat::{activate_tile_chat, sync_chat_for_tile};
use super::controls::show_tile_overlay;
use super::ffi::{
    G_SOURCE_REMOVE, GtkButton, GtkGLArea, MPV_MAINLOOP_PRIORITY,
    g_atomic_int_compare_and_exchange, g_atomic_int_set, g_free, g_strdup, g_strdup_printf,
    gtk_gl_area_queue_render, gtk_widget_get_realized, gtk_widget_set_sensitive,
    gtk_widget_set_visible,
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
use super::{FALSE, PLAYER_STARTING_STREAM_STATUS, StreamTile, TRUE};
use crate::glib_strings::is_nonempty;
use crate::player::channel_switcher_overlay::channel_switcher_overlay_show_at;
use crate::player::controls::player_volume_update_mute_button;
use crate::player::session::{
    player_session_drop_buffers, player_session_free, player_session_get_channel,
    player_session_get_label, player_session_get_mpv, player_session_is_playing,
    player_session_is_ready, player_session_load_stream, player_session_new,
    player_session_set_hwdec_enabled, player_session_set_ytdl_raw_options, player_session_stop,
};
use crate::settings::{
    AppSettingsChannel, app_settings_get_hwdec_enabled, app_settings_get_twitch_playback_auth_token,
};
use crate::twitch::channel::normalize_twitch_channel;
use gtk4::glib;
use libmpv2::{Mpv, events::Event, mpv_end_file_reason};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::Ordering;

pub(super) unsafe fn tile_mpv(tile: *mut StreamTile) -> Option<Rc<Mpv>> {
    unsafe { player_session_get_mpv((*tile).session) }
}

// Drain queued mpv events without waiting for new ones.
pub(super) unsafe extern "C" fn process_mpv_events(user_data: *mut c_void) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;

        g_atomic_int_set(&mut (*tile).event_queued, 0);

        if (*(*tile).app).closing.load(Ordering::Acquire) {
            return G_SOURCE_REMOVE;
        }
        let Some(mpv) = tile_mpv(tile) else {
            return G_SOURCE_REMOVE;
        };
        while let Some(event) = mpv.wait_event(0.0) {
            match event {
                Ok(Event::StartFile) => set_tile_status(tile, c"Loading".as_ptr()),
                Ok(Event::FileLoaded) => set_tile_status(tile, c"Playback running".as_ptr()),
                Ok(Event::EndFile(mpv_end_file_reason::Eof)) => {
                    set_tile_status(tile, c"Stopped".as_ptr())
                }
                Ok(Event::EndFile(mpv_end_file_reason::Error)) | Err(_) => {
                    set_tile_status(tile, c"Stream could not be played".as_ptr());
                }
                Ok(Event::LogMessage {
                    prefix,
                    text,
                    ..
                }) => {
                    glib::g_debug!("twitch-player", "mpv[{prefix}][{}]: {text}", (*tile).index);
                }
                Ok(Event::Shutdown) => return G_SOURCE_REMOVE,
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
