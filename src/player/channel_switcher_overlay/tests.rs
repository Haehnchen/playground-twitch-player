use super::actions::{activate_first_matching_channel, preview_matches_filter};
use super::ffi::*;
use super::widgets::queue_preview_card_batch;
use super::{
    ChannelSwitcherOverlay, channel_switcher_overlay_free, channel_switcher_overlay_hide,
    channel_switcher_overlay_ref, channel_switcher_overlay_unref,
};
use crate::settings::AppSettingsChannel;
use crate::twitch::stream_info::TwitchStreamPreview;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

static MAIN_CONTEXT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

unsafe extern "C" {
    fn g_main_context_iteration(context: *mut c_void, may_block: c_int) -> c_int;
}

unsafe extern "C" fn keep_redraw_ready(_data: *mut c_void) -> c_int {
    G_SOURCE_CONTINUE
}

fn empty_switcher() -> *mut ChannelSwitcherOverlay {
    Box::into_raw(Box::new(ChannelSwitcherOverlay::new(
        ptr::null_mut(),
        None,
        ptr::null_mut(),
        None,
        ptr::null_mut(),
    )))
}

fn sample_preview() -> TwitchStreamPreview {
    TwitchStreamPreview {
        channel: c"test_channel".as_ptr() as *mut c_char,
        display_name: c"Test Channel".as_ptr() as *mut c_char,
        title: c"Grüße aus Köln".as_ptr() as *mut c_char,
        avatar_url: ptr::null_mut(),
        preview_url: ptr::null_mut(),
        started_at: ptr::null_mut(),
        category_name: c"Games + Demos".as_ptr() as *mut c_char,
        viewer_count: 1,
    }
}

#[test]
fn search_matches_all_preview_fields_case_insensitively() {
    let mut preview = sample_preview();
    for filter in [
        c"",
        c"TEST_CHANNEL",
        c"Test Channel",
        c"GRÜSSE",
        c"Köln",
        c"demos",
    ] {
        assert!(
            unsafe { preview_matches_filter(&mut preview, filter.as_ptr()) },
            "{filter:?}"
        );
    }
    for filter in [c"missing", c"offline", c"demos tv"] {
        assert!(
            !unsafe { preview_matches_filter(&mut preview, filter.as_ptr()) },
            "{filter:?}"
        );
    }
}

#[test]
fn owner_release_waits_for_pending_callback_reference() {
    unsafe {
        let switcher = empty_switcher();
        let pending = channel_switcher_overlay_ref(switcher);

        channel_switcher_overlay_free(switcher);

        assert!((*pending).closing.load(Ordering::Acquire));
        assert_eq!((*pending).ref_count.load(Ordering::Acquire), 1);

        channel_switcher_overlay_unref(pending);
    }
}

#[test]
fn preview_batches_progress_while_redraw_is_ready() {
    let _guard = MAIN_CONTEXT_TEST_LOCK.lock().unwrap();
    unsafe {
        let switcher = empty_switcher();
        // GDK redraws run at priority 120. Keep one ready to simulate busy playback.
        let redraw = g_idle_add_full(120, Some(keep_redraw_ready), ptr::null_mut(), None);
        (*switcher).preview_card_build_source = queue_preview_card_batch(switcher);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
        while (*switcher).preview_card_build_source != 0 && std::time::Instant::now() < deadline {
            g_main_context_iteration(ptr::null_mut(), FALSE);
        }
        // The empty switcher makes the real batch callback stop before touching GTK.
        let completed = (*switcher).preview_card_build_source == 0;
        g_source_remove(redraw);
        channel_switcher_overlay_free(switcher);
        assert!(completed, "preview work was starved by continuous redraws");
    }
}

#[test]
fn hiding_cancels_pending_preview_batches() {
    let _guard = MAIN_CONTEXT_TEST_LOCK.lock().unwrap();
    unsafe {
        let switcher = empty_switcher();
        (*switcher).preview_card_build_source = queue_preview_card_batch(switcher);
        assert_eq!((*switcher).ref_count.load(Ordering::Acquire), 2);

        channel_switcher_overlay_hide(switcher);

        assert_eq!((*switcher).preview_card_build_source, 0);
        assert_eq!((*switcher).ref_count.load(Ordering::Acquire), 1);
        channel_switcher_overlay_free(switcher);
    }
}

#[test]
fn search_activation_works_before_cards_are_built() {
    unsafe extern "C" fn activated(channel: *const AppSettingsChannel, data: *mut c_void) {
        unsafe {
            let selected = &mut *(data as *mut Option<(Vec<u8>, Vec<u8>)>);
            *selected = Some((
                CStr::from_ptr((*channel).channel).to_bytes().to_vec(),
                CStr::from_ptr((*channel).label).to_bytes().to_vec(),
            ));
        }
    }

    unsafe {
        let switcher = empty_switcher();
        let mut selected: Option<(Vec<u8>, Vec<u8>)> = None;
        let mut preview = sample_preview();
        (*switcher).previews = gtk4::glib::ffi::g_ptr_array_new_with_free_func(None).cast();
        gtk4::glib::ffi::g_ptr_array_add(
            (*switcher).previews.cast(),
            &mut preview as *mut _ as *mut c_void,
        );
        (*switcher).activate_callback = Some(activated);
        (*switcher).user_data = &mut selected as *mut _ as *mut c_void;
        assert_eq!((*switcher).preview_cards.len(), 0);

        activate_first_matching_channel(switcher);

        assert_eq!(
            selected,
            Some((b"test_channel".to_vec(), b"Test Channel".to_vec()))
        );
        channel_switcher_overlay_free(switcher);
    }
}
