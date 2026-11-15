//! Picker styles.

use super::ffi::*;
use std::ffi::{CString, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

static CSS_INSTALLED: AtomicBool = AtomicBool::new(false);

pub(super) unsafe fn install_css() {
    unsafe {
        if CSS_INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }

        let provider = gtk_css_provider_new();
        let css = CString::new(
            ".channel-switcher-panel {\
          background: rgba(12, 12, 14, 0.88);\
          color: #ffffff;\
          padding: 8px 8px 10px 8px;\
          border-radius: 6px;\
          box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);\
        }\
        .channel-switcher-backdrop {\
          background: transparent;\
        }\
        .channel-switcher-header {\
          margin-bottom: 4px;\
        }\
        .channel-switcher-search {\
          background: rgba(255, 255, 255, 0.08);\
          color: #ffffff;\
          border-color: rgba(255, 255, 255, 0.10);\
          outline-color: transparent;\
          box-shadow: none;\
          font-size: 12px;\
          min-height: 22px;\
          min-width: 165px;\
          padding: 1px 7px;\
          border-radius: 4px;\
        }\
        .channel-switcher-header-separator {\
          color: rgba(255, 255, 255, 0.34);\
          font-size: 13px;\
          margin-left: 1px;\
          margin-right: 1px;\
        }\
        .channel-switcher-open-entry {\
          background: rgba(255, 255, 255, 0.08);\
          color: #ffffff;\
          border-color: rgba(255, 255, 255, 0.10);\
          outline-color: transparent;\
          box-shadow: none;\
          font-size: 12px;\
          min-height: 22px;\
          min-width: 165px;\
          padding: 1px 7px;\
          border-radius: 4px;\
        }\
        .channel-switcher-search selection {\
          background: rgba(145, 70, 255, 0.50);\
          color: #ffffff;\
        }\
        .channel-switcher-action,\
        .channel-switcher-close {\
          background: transparent;\
          background-image: none;\
          color: #ffffff;\
          border-color: transparent;\
          outline-color: transparent;\
          box-shadow: none;\
          min-width: 24px;\
          min-height: 22px;\
          padding: 1px 4px;\
          border-radius: 4px;\
        }\
        .channel-switcher-action:hover {\
          background: rgba(255, 255, 255, 0.16);\
        }\
        .channel-switcher-close:hover {\
          background: rgba(170, 36, 36, 0.90);\
          background-image: none;\
        }\
        .channel-switcher-scroller,\
        .channel-switcher-scroller viewport {\
          background: transparent;\
        }\
        .channel-switcher-grid {\
          background: transparent;\
        }\
        .channel-switcher-item {\
          background: rgba(255, 255, 255, 0.06);\
          color: #ffffff;\
          border-color: transparent;\
          outline-color: transparent;\
          box-shadow: none;\
          border-radius: 5px;\
          margin: 0;\
          padding: 5px;\
        }\
        .channel-switcher-item:hover {\
          background: rgba(255, 255, 255, 0.13);\
        }\
        .channel-switcher-preview {\
          background: rgba(255, 255, 255, 0.08);\
          border-radius: 4px;\
          min-width: 226px;\
          min-height: 127px;\
        }\
        .channel-switcher-avatar {\
          background: rgba(0, 0, 0, 0.55);\
          border-radius: 999px;\
          min-width: 24px;\
          min-height: 24px;\
          margin: 0;\
        }\
        .channel-switcher-name {\
          color: #ffffff;\
          font-weight: 700;\
          font-size: 13px;\
        }\
        .channel-switcher-title {\
          color: rgba(255, 255, 255, 0.86);\
          font-size: 12px;\
        }\
        .channel-switcher-meta {\
          color: rgba(255, 255, 255, 0.66);\
          font-size: 11px;\
        }\
        .channel-switcher-status {\
          color: rgba(255, 255, 255, 0.72);\
          padding: 12px;\
        }",
        )
        .expect("static CSS has no NUL bytes");
        gtk_css_provider_load_from_string(provider, css.as_ptr());
        gtk_style_context_add_provider_for_display(
            gdk_display_get_default(),
            provider as *mut GtkStyleProvider,
            GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        g_object_unref(provider as *mut c_void);
    }
}
