use super::c_app_id;
use super::ffi::{
    g_build_filename, g_canonicalize_filename, g_clear_error, g_file_read_link,
    g_file_set_contents, g_free, g_get_current_dir, g_get_user_data_dir, g_mkdir_with_parents,
    g_path_is_absolute, g_strdup, g_strdup_printf, gdk_display_get_default,
    gtk_icon_theme_add_search_path, gtk_icon_theme_get_for_display,
    gtk_window_set_default_icon_name,
};
use super::runtime::{error_message, log_debug_error, log_debug_static};
use crate::glib_strings::dup_bytes;
use crate::settings::GError;
use std::ffi::{CStr, c_char, c_void};
use std::ptr;
pub(super) const APP_ICON_SVG: &[u8] =
    include_bytes!("../../data/icons/hicolor/scalable/apps/local.twitch-player.svg");

pub(super) unsafe fn install_app_icon() {
    unsafe {
        let theme = gtk_icon_theme_get_for_display(gdk_display_get_default());
        let icons_dir = g_build_filename(
            g_get_user_data_dir(),
            c"icons".as_ptr(),
            ptr::null::<c_char>(),
        );
        gtk_icon_theme_add_search_path(theme, icons_dir);
        g_free(icons_dir as *mut c_void);
        gtk_window_set_default_icon_name(c_app_id());
    }
}

pub(super) unsafe fn get_executable_path(argv0: *const c_char) -> *mut c_char {
    unsafe {
        let link_path = g_file_read_link(c"/proc/self/exe".as_ptr(), ptr::null_mut());
        if !link_path.is_null() {
            return link_path;
        }

        if g_path_is_absolute(argv0) != 0 {
            return g_strdup(argv0);
        }

        let cwd = g_get_current_dir();
        let result = g_canonicalize_filename(argv0, cwd);
        g_free(cwd as *mut c_void);
        result
    }
}

pub(super) unsafe fn quote_desktop_exec_path(path: *const c_char) -> *mut c_char {
    unsafe {
        let bytes = if path.is_null() {
            &[][..]
        } else {
            CStr::from_ptr(path).to_bytes()
        };
        let mut quoted = Vec::with_capacity(bytes.len() + 2);
        quoted.push(b'"');
        for byte in bytes {
            if matches!(*byte, b'"' | b'\\' | b'`' | b'$') {
                quoted.push(b'\\');
            }
            quoted.push(*byte);
        }
        quoted.push(b'"');
        dup_bytes(&quoted)
    }
}

pub(super) unsafe fn write_user_desktop_identity(argv0: *const c_char) {
    unsafe {
        let applications_dir = g_build_filename(
            g_get_user_data_dir(),
            c"applications".as_ptr(),
            ptr::null::<c_char>(),
        );
        let icons_dir = g_build_filename(
            g_get_user_data_dir(),
            c"icons".as_ptr(),
            c"hicolor".as_ptr(),
            c"scalable".as_ptr(),
            c"apps".as_ptr(),
            ptr::null::<c_char>(),
        );
        let desktop_path = g_build_filename(
            applications_dir,
            c"local.twitchplayer.desktop".as_ptr(),
            ptr::null::<c_char>(),
        );
        let icon_path = g_build_filename(
            icons_dir,
            c"local.twitchplayer.svg".as_ptr(),
            ptr::null::<c_char>(),
        );
        let exec_path = get_executable_path(argv0);
        let quoted_exec = quote_desktop_exec_path(exec_path);
        let mut error: *mut GError = ptr::null_mut();

        if g_mkdir_with_parents(applications_dir, 0o700) < 0
            || g_mkdir_with_parents(icons_dir, 0o700) < 0
        {
            log_debug_static(c"could not create user desktop/icon directories".as_ptr());
            g_free(applications_dir as *mut c_void);
            g_free(icons_dir as *mut c_void);
            g_free(desktop_path as *mut c_void);
            g_free(icon_path as *mut c_void);
            g_free(exec_path as *mut c_void);
            g_free(quoted_exec as *mut c_void);
            return;
        }

        if g_file_set_contents(
            icon_path,
            APP_ICON_SVG.as_ptr() as *const c_char,
            APP_ICON_SVG.len() as isize,
            &mut error,
        ) == 0
        {
            log_debug_error(
                c"could not write user app icon: %s".as_ptr(),
                error_message(error),
            );
            g_clear_error(&mut error);
            g_free(applications_dir as *mut c_void);
            g_free(icons_dir as *mut c_void);
            g_free(desktop_path as *mut c_void);
            g_free(icon_path as *mut c_void);
            g_free(exec_path as *mut c_void);
            g_free(quoted_exec as *mut c_void);
            return;
        }

        let desktop = g_strdup_printf(
            c"[Desktop Entry]\nType=Application\nName=Twitch Player\nExec=%s %%u\nIcon=%s\nTerminal=false\nCategories=AudioVideo;Player;Network;\nStartupNotify=true\nStartupWMClass=%s\n".as_ptr(),
            quoted_exec,
            icon_path,
            c_app_id(),
        );

        if g_file_set_contents(desktop_path, desktop, -1, &mut error) == 0 {
            log_debug_error(
                c"could not write desktop entry: %s".as_ptr(),
                error_message(error),
            );
            g_clear_error(&mut error);
        }

        g_free(desktop as *mut c_void);
        g_free(applications_dir as *mut c_void);
        g_free(icons_dir as *mut c_void);
        g_free(desktop_path as *mut c_void);
        g_free(icon_path as *mut c_void);
        g_free(exec_path as *mut c_void);
        g_free(quoted_exec as *mut c_void);
    }
}
