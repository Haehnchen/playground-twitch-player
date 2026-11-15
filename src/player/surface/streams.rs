use super::chat::sync_chat_for_tile;
use super::controls::show_tile_overlay;
use super::ffi::{
    G_IO_ERROR_CANCELLED, G_LOG_LEVEL_DEBUG, G_SOURCE_CONTINUE, G_SOURCE_REMOVE, GObject,
    GtkButton, GtkLabel, GtkPopover, g_cancellable_cancel, g_cancellable_new, g_clear_error,
    g_error_matches, g_free, g_io_error_quark, g_log, g_object_get_data, g_ptr_array_unref,
    gtk_label_set_text, gtk_popover_popdown, gtk_popover_popup, gtk_widget_set_tooltip_text,
};
use super::playback::{load_tile_stream, set_tile_status, update_tile_empty_state};
use super::runtime::{
    clear_object, tile_is_closing, tile_ref, tile_signal_data_free, tile_signal_data_ref,
};
use super::stream_details::{
    hide_tile_twitch_playback_panel, set_tile_twitch_playback_info,
    set_tile_twitch_playback_loading, set_tile_twitch_playback_unavailable,
};
use super::{
    FALSE, G_LOG_DOMAIN, MAX_TILES, PLAYER_STARTING_STREAM_STATUS, PlayerSurface,
    StreamQualityCallbackData, StreamTile, StreamTitleCallbackData, TRUE, player_surface_unref,
};
use crate::glib_strings::is_nonempty;
use crate::player::controls::player_footer_stream_info_set;
use crate::player::session::{player_session_is_playing, player_session_load_stream};
use crate::player::stream_quality::{
    player_stream_quality_state_begin_fetch, player_stream_quality_state_cache_is_valid,
    player_stream_quality_state_cancel_fetch, player_stream_quality_state_clear,
    player_stream_quality_state_finish_fetch, player_stream_quality_state_mark_fetched,
    player_stream_quality_state_reset_selection, player_stream_quality_state_select,
    player_stream_quality_state_select_auto,
};
use crate::player::stream_settings::{
    QualitySelectionCallbacks, player_stream_settings_quality_list_populate,
};
use crate::settings::app_settings_get_twitch_playback_auth_token;
use crate::twitch::stream_info::{
    GAsyncResult, GError, TwitchStreamQuality, twitch_current_stream_free,
    twitch_playback_info_empty, twitch_stream_info_fetch_current_stream_async,
    twitch_stream_info_fetch_current_stream_finish,
    twitch_stream_info_fetch_stream_qualities_async,
    twitch_stream_info_fetch_stream_qualities_finish,
    twitch_stream_info_format_current_stream_metadata,
    twitch_stream_info_format_current_stream_title,
};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

pub(super) const PLAYER_EMPTY_STREAM_LABEL: *const c_char = c"Select".as_ptr();

pub(super) unsafe fn stream_title_callback_data_free(data: *mut StreamTitleCallbackData) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data);
        player_surface_unref((*data.tile).app);
    }
}

pub(super) unsafe fn set_tile_stream_title(
    tile: *mut StreamTile,
    title: *const c_char,
    metadata: *const c_char,
) {
    unsafe {
        player_footer_stream_info_set((*tile).stream_info, title, metadata);
    }
}

pub(super) unsafe fn reset_tile_stream_title(tile: *mut StreamTile) {
    unsafe {
        (*tile).title_generation = (*tile).title_generation.wrapping_add(1);
        if !(*tile).title_cancel.is_null() {
            g_cancellable_cancel((*tile).title_cancel);
            clear_object(&mut (*tile).title_cancel);
        }
        (*tile).title_fetch_in_progress = FALSE;
        set_tile_stream_title(tile, c"".as_ptr(), c"".as_ptr());
    }
}

pub(super) unsafe extern "C" fn on_tile_title_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let data = user_data as *mut StreamTitleCallbackData;
        let tile = (*data).tile;
        let mut error: *mut GError = ptr::null_mut();
        let stream = twitch_stream_info_fetch_current_stream_finish(result, &mut error);

        if (*data).generation != (*tile).title_generation {
            g_clear_error(&mut error);
            twitch_current_stream_free(stream);
            stream_title_callback_data_free(data);
            return;
        }

        (*tile).title_fetch_in_progress = FALSE;
        clear_object(&mut (*tile).title_cancel);

        if (*(*tile).app).closing.load(Ordering::Acquire)
            || player_session_is_playing((*tile).session) == 0
        {
            g_clear_error(&mut error);
            twitch_current_stream_free(stream);
            stream_title_callback_data_free(data);
            return;
        }

        if !error.is_null() {
            if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
                g_log(
                    G_LOG_DOMAIN.as_ptr() as *const c_char,
                    G_LOG_LEVEL_DEBUG,
                    c"surface stream title fetch failed: %s".as_ptr(),
                    (*error).message,
                );
            }
            g_clear_error(&mut error);
            twitch_current_stream_free(stream);
            stream_title_callback_data_free(data);
            return;
        }

        let title = twitch_stream_info_format_current_stream_title(stream);
        let metadata = twitch_stream_info_format_current_stream_metadata(stream);
        set_tile_stream_title(tile, title, metadata);
        g_free(title as *mut c_void);
        g_free(metadata as *mut c_void);
        twitch_current_stream_free(stream);
        stream_title_callback_data_free(data);
    }
}

pub(super) unsafe fn request_tile_title_update(tile: *mut StreamTile, force: c_int) {
    unsafe {
        if (*(*tile).app).closing.load(Ordering::Acquire)
            || player_session_is_playing((*tile).session) == 0
            || !is_nonempty((*tile).channel)
        {
            return;
        }
        if (*tile).title_fetch_in_progress != 0 && force == 0 {
            return;
        }

        if force != 0 {
            reset_tile_stream_title(tile);
        }

        let data = Box::into_raw(Box::new(StreamTitleCallbackData {
            tile: tile_ref(tile),
            generation: (*tile).title_generation.wrapping_add(1),
        }));
        (*tile).title_generation = (*data).generation;

        (*tile).title_cancel = g_cancellable_new();
        (*tile).title_fetch_in_progress = TRUE;

        twitch_stream_info_fetch_current_stream_async(
            (*tile).channel,
            (*tile).title_cancel,
            Some(on_tile_title_fetched),
            data as *mut c_void,
        );
    }
}

pub(super) unsafe extern "C" fn refresh_surface_stream_titles(user_data: *mut c_void) -> c_int {
    unsafe {
        let state = user_data as *mut PlayerSurface;

        if (*state).closing.load(Ordering::Acquire) {
            (*state).title_refresh_source = 0;
            return G_SOURCE_REMOVE;
        }

        for i in 0..MAX_TILES {
            request_tile_title_update(&mut (*state).tiles[i], FALSE);
        }

        G_SOURCE_CONTINUE
    }
}

pub(super) unsafe fn update_tile_channel_label(tile: *mut StreamTile) {
    unsafe {
        if (*tile).channel_label.is_null() {
            return;
        }

        let label = if is_nonempty((*tile).label) {
            (*tile).label
        } else {
            PLAYER_EMPTY_STREAM_LABEL as *mut c_char
        };
        gtk_label_set_text((*tile).channel_label as *mut GtkLabel, label);
        gtk_widget_set_tooltip_text(
            (*tile).channel_label,
            if is_nonempty((*tile).label) {
                (*tile).label
            } else {
                ptr::null_mut()
            },
        );
    }
}

pub(super) const STREAM_QUALITY_CACHE_SECONDS: c_uint = 2 * 60;

pub(super) unsafe fn stream_quality_callback_data_free(data: *mut StreamQualityCallbackData) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data);
        player_surface_unref((*data.tile).app);
    }
}

pub(super) unsafe fn clear_tile_stream_qualities(tile: *mut StreamTile) {
    unsafe {
        player_stream_quality_state_clear(&mut (*tile).stream_quality);
    }
}

pub(super) unsafe fn reset_tile_quality_selection(tile: *mut StreamTile) {
    unsafe {
        player_stream_quality_state_reset_selection(&mut (*tile).stream_quality);
    }
}

pub(super) unsafe fn tile_qualities_cache_is_valid(tile: *mut StreamTile) -> bool {
    unsafe {
        player_stream_quality_state_cache_is_valid(
            &mut (*tile).stream_quality,
            STREAM_QUALITY_CACHE_SECONDS,
        ) != 0
    }
}

pub(super) unsafe fn reload_tile_stream_with_quality(
    tile: *mut StreamTile,
    quality: *const TwitchStreamQuality,
) {
    unsafe {
        if quality.is_null() || !is_nonempty((*quality).url) || !is_nonempty((*tile).channel) {
            return;
        }

        player_stream_quality_state_select(&mut (*tile).stream_quality, quality);

        set_tile_status(tile, PLAYER_STARTING_STREAM_STATUS);
        player_session_load_stream(
            (*tile).session,
            (*quality).url,
            (*tile).label,
            (*tile).channel,
        );
        update_tile_empty_state(tile);
        sync_chat_for_tile(tile);
        request_tile_title_update(tile, TRUE);
    }
}

pub(super) unsafe fn reload_tile_stream_auto(tile: *mut StreamTile) {
    unsafe {
        player_stream_quality_state_select_auto(&mut (*tile).stream_quality);
        load_tile_stream(tile);
    }
}

pub(super) unsafe extern "C" fn on_tile_quality_auto_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        reload_tile_stream_auto(tile);
        if !(*tile).stream_settings_popover.is_null() {
            gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
        }
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_tile_quality_button_clicked(
    button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        let quality = g_object_get_data(button as *mut GObject, c"stream-quality".as_ptr())
            as *const TwitchStreamQuality;

        reload_tile_stream_with_quality(tile, quality);
        if !(*tile).stream_settings_popover.is_null() {
            gtk_popover_popdown((*tile).stream_settings_popover as *mut GtkPopover);
        }
        show_tile_overlay(tile);
    }
}

pub(super) unsafe extern "C" fn on_tile_stream_qualities_fetched(
    _source_object: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let data = user_data as *mut StreamQualityCallbackData;
        let tile = (*data).tile;
        let mut error: *mut GError = ptr::null_mut();
        let mut playback_info = twitch_playback_info_empty();
        let qualities = twitch_stream_info_fetch_stream_qualities_finish(
            result,
            &mut playback_info,
            &mut error,
        );

        if (*data).generation != (*tile).stream_quality.generation {
            if !qualities.is_null() {
                g_ptr_array_unref(qualities);
            }
            g_clear_error(&mut error);
            stream_quality_callback_data_free(data);
            return;
        }

        player_stream_quality_state_finish_fetch(&mut (*tile).stream_quality, qualities);

        if !error.is_null() {
            if g_error_matches(error, g_io_error_quark(), G_IO_ERROR_CANCELLED) == 0 {
                gtk_label_set_text(
                    (*tile).quality_status_label as *mut GtkLabel,
                    c"Qualities unavailable".as_ptr(),
                );
                g_log(
                    G_LOG_DOMAIN.as_ptr() as *const c_char,
                    G_LOG_LEVEL_DEBUG,
                    c"surface stream quality fetch failed: %s".as_ptr(),
                    (*error).message,
                );
                set_tile_twitch_playback_unavailable(tile);
            }
            g_clear_error(&mut error);
            stream_quality_callback_data_free(data);
            return;
        }

        player_stream_quality_state_mark_fetched(&mut (*tile).stream_quality);
        set_tile_twitch_playback_info(tile, &playback_info);
        populate_tile_quality_buttons(tile);
        stream_quality_callback_data_free(data);
    }
}

pub(super) unsafe fn request_tile_qualities_update(tile: *mut StreamTile, force: c_int) {
    unsafe {
        if (*(*tile).app).closing.load(Ordering::Acquire) || !is_nonempty((*tile).channel) {
            return;
        }
        if (*tile).stream_quality.fetch_in_progress != 0 && force == 0 {
            return;
        }
        if force == 0 && tile_qualities_cache_is_valid(tile) {
            populate_tile_quality_buttons(tile);
            return;
        }

        if force != 0 {
            player_stream_quality_state_cancel_fetch(&mut (*tile).stream_quality);
        }

        set_tile_twitch_playback_loading(tile);
        gtk_label_set_text(
            (*tile).quality_status_label as *mut GtkLabel,
            c"Loading...".as_ptr(),
        );

        let data = Box::into_raw(Box::new(StreamQualityCallbackData {
            tile: tile_ref(tile),
            generation: player_stream_quality_state_begin_fetch(&mut (*tile).stream_quality),
        }));

        twitch_stream_info_fetch_stream_qualities_async(
            (*tile).channel,
            app_settings_get_twitch_playback_auth_token((*(*tile).app).settings),
            (*tile).stream_quality.cancel,
            Some(on_tile_stream_qualities_fetched),
            data as *mut c_void,
        );
    }
}

pub(super) unsafe fn populate_tile_quality_buttons(tile: *mut StreamTile) {
    unsafe {
        player_stream_settings_quality_list_populate(
            (*tile).quality_list_box,
            (*tile).quality_status_label,
            (*tile).stream_quality.qualities,
            (*tile).stream_quality.selected_url,
            (*tile).stream_quality.selected_label,
            QualitySelectionCallbacks {
                quality_clicked: on_tile_quality_button_clicked as *const c_void,
                auto_clicked: on_tile_quality_auto_clicked as *const c_void,
                user_data: tile as *mut c_void,
                data_ref: Some(tile_signal_data_ref),
                data_free: Some(tile_signal_data_free),
            },
        );
    }
}

pub(super) unsafe extern "C" fn on_tile_stream_settings_clicked(
    _button: *mut GtkButton,
    user_data: *mut c_void,
) {
    unsafe {
        let tile = user_data as *mut StreamTile;

        if tile_is_closing(tile) || (*tile).stream_settings_popover.is_null() {
            return;
        }
        hide_tile_twitch_playback_panel(tile);
        if player_session_is_playing((*tile).session) == 0 || !is_nonempty((*tile).channel) {
            show_tile_overlay(tile);
            return;
        }

        request_tile_qualities_update(tile, FALSE);
        gtk_popover_popup((*tile).stream_settings_popover as *mut GtkPopover);
        show_tile_overlay(tile);
    }
}
