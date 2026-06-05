use std::ffi::{c_char, c_void};

const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: u32 = 600;

const OVERLAY_CSS: &str = r#"
.top-overlay-controls {
  margin: 6px;
}
.overlay-icon-button {
  background: rgba(0, 0, 0, 0.58);
  color: white;
  border-color: transparent;
  outline-color: transparent;
  box-shadow: none;
  min-width: 30px;
  min-height: 28px;
  padding: 3px 7px;
}
.overlay-icon-button:hover {
  background: rgba(54, 54, 54, 0.90);
}
.empty-stream-button {
  background: transparent;
  color: rgba(255, 255, 255, 0.50);
  border-color: transparent;
  outline-color: transparent;
  box-shadow: none;
  min-width: 52px;
  min-height: 52px;
  padding: 0;
  opacity: 0.50;
}
.empty-stream-button:hover {
  background: transparent;
  color: rgba(255, 255, 255, 0.65);
  opacity: 0.65;
}
.empty-stream-button-visible {
  min-width: 30px;
  min-height: 30px;
}
button.close-button:hover,
.tile-footer button.tile-close-button:hover {
  background: rgba(170, 36, 36, 0.90);
}
"#;

const FOOTER_CSS: &str = r#"
.player-footer .overlay-icon-button,
.player-footer button.overlay-icon-button {
  background: transparent;
  background-color: transparent;
  background-image: none;
  border: none;
  border-color: transparent;
  outline-color: transparent;
  box-shadow: none;
  color: rgba(255, 255, 255, 0.88);
  min-width: 24px;
  min-height: 22px;
  padding: 2px 3px;
}
.player-footer .overlay-icon-button:hover,
.player-footer button.overlay-icon-button:hover {
  background: rgba(255, 255, 255, 0.14);
  background-image: none;
  color: white;
}
.player-footer .volume-mute-button {
  margin-right: 0;
}
.stream-settings-popover contents {
  background: rgba(28, 28, 28, 0.98);
  padding: 0;
  margin: 0;
  border: none;
  border-radius: 4px;
  box-shadow: none;
}
.stream-settings-menu {
  background: rgba(28, 28, 28, 0.98);
  padding: 6px;
  min-width: 150px;
  margin: 0;
}
.stream-settings-heading {
  color: rgba(255, 255, 255, 0.66);
  font-size: 11px;
  font-weight: 700;
  margin-top: 1px;
  margin-bottom: 0;
}
.stream-settings-status {
  color: rgba(255, 255, 255, 0.64);
  font-size: 11px;
  margin-top: 1px;
  margin-bottom: 0;
}
.stream-settings-divider {
  background: rgba(255, 255, 255, 0.18);
  min-height: 1px;
  margin: 1px 0;
}
.stream-settings-item {
  background: transparent;
  color: white;
  font-size: 12px;
  border-color: transparent;
  outline-color: transparent;
  box-shadow: none;
  border-radius: 4px;
  margin: 0;
  min-height: 0;
  padding: 3px 6px;
}
.stream-settings-item label {
  margin: 0;
  padding: 0;
}
.stream-settings-item:hover {
  background: rgba(74, 74, 74, 0.98);
}
.stream-settings-item-selected {
  background: rgba(255, 255, 255, 0.16);
}
.player-footer scale.volume-scale,
.player-footer .volume-scale {
  margin-left: 0;
  margin-right: 0;
  padding-left: 0;
  padding-right: 0;
  outline: none;
  outline-color: transparent;
  box-shadow: none;
}
.player-footer scale.volume-scale:hover,
.player-footer scale.volume-scale:focus,
.player-footer scale.volume-scale:active,
.player-footer .volume-scale:hover,
.player-footer .volume-scale:focus,
.player-footer .volume-scale:active {
  outline: none;
  outline-color: transparent;
  box-shadow: none;
}
.player-footer scale.volume-scale trough {
  background: rgba(255, 255, 255, 0.20);
  background-color: rgba(255, 255, 255, 0.20);
  background-image: none;
  min-height: 3px;
}
scale.volume-scale trough,
.volume-scale trough,
scale.volume-scale > trough,
.volume-scale > trough {
  background: rgba(255, 255, 255, 0.20);
  background-color: rgba(255, 255, 255, 0.20);
  background-image: none;
  min-height: 3px;
}
scale.volume-scale > trough > highlight,
.volume-scale > trough > highlight,
.player-footer scale.volume-scale trough highlight,
.player-footer scale.volume-scale:hover trough highlight,
.player-footer scale.volume-scale:focus trough highlight,
.player-footer scale.volume-scale:active trough highlight,
.player-footer .volume-scale trough highlight,
.player-footer .volume-scale:hover trough highlight,
.player-footer .volume-scale:focus trough highlight,
.player-footer .volume-scale:active trough highlight,
.player-footer scale.volume-scale trough fill,
.player-footer scale.volume-scale:hover trough fill,
.player-footer scale.volume-scale:focus trough fill,
.player-footer scale.volume-scale:active trough fill,
.player-footer .volume-scale trough fill,
.player-footer .volume-scale:hover trough fill,
.player-footer .volume-scale:focus trough fill,
.player-footer .volume-scale:active trough fill,
.player-footer scale.volume-scale highlight,
.player-footer scale.volume-scale:hover highlight,
.player-footer scale.volume-scale:focus highlight,
.player-footer scale.volume-scale:active highlight,
.player-footer .volume-scale highlight,
.player-footer .volume-scale:hover highlight,
.player-footer .volume-scale:focus highlight,
.player-footer .volume-scale:active highlight,
.player-footer scale.volume-scale fill,
.player-footer scale.volume-scale:hover fill,
.player-footer scale.volume-scale:focus fill,
.player-footer scale.volume-scale:active fill,
.player-footer .volume-scale fill,
.player-footer .volume-scale:hover fill,
.player-footer .volume-scale:focus fill,
.player-footer .volume-scale:active fill {
  background: rgba(255, 255, 255, 0.72);
  background-color: rgba(255, 255, 255, 0.72);
  background-image: none;
  box-shadow: none;
}
scale.volume-scale trough highlight,
scale.volume-scale:hover trough highlight,
scale.volume-scale:focus trough highlight,
scale.volume-scale:active trough highlight,
.volume-scale trough highlight,
.volume-scale:hover trough highlight,
.volume-scale:focus trough highlight,
.volume-scale:active trough highlight,
scale.volume-scale highlight,
scale.volume-scale:hover highlight,
scale.volume-scale:focus highlight,
scale.volume-scale:active highlight,
.volume-scale highlight,
.volume-scale:hover highlight,
.volume-scale:focus highlight,
.volume-scale:active highlight,
scale.volume-scale trough fill,
.volume-scale trough fill,
scale.volume-scale fill,
.volume-scale fill {
  background: rgba(255, 255, 255, 0.72);
  background-color: rgba(255, 255, 255, 0.72);
  background-image: none;
  box-shadow: none;
}
scale.volume-scale > trough > slider,
.volume-scale > trough > slider,
.player-footer scale.volume-scale slider,
.player-footer scale.volume-scale:hover slider,
.player-footer scale.volume-scale:focus slider,
.player-footer scale.volume-scale:active slider,
.player-footer .volume-scale slider,
.player-footer .volume-scale:hover slider,
.player-footer .volume-scale:focus slider,
.player-footer .volume-scale:active slider {
  background: rgba(255, 255, 255, 0.95);
  background-color: rgba(255, 255, 255, 0.95);
  background-image: none;
  border-color: rgba(0, 0, 0, 0.45);
  outline: none;
  outline-color: transparent;
  box-shadow: none;
  min-width: 10px;
  min-height: 10px;
  margin-top: -4px;
  margin-bottom: -4px;
  margin-left: 0;
  margin-right: 0;
}
scale.volume-scale slider,
scale.volume-scale:hover slider,
scale.volume-scale:focus slider,
scale.volume-scale:active slider,
.volume-scale slider,
.volume-scale:hover slider,
.volume-scale:focus slider,
.volume-scale:active slider {
  background: rgba(255, 255, 255, 0.95);
  background-color: rgba(255, 255, 255, 0.95);
  background-image: none;
  border-color: rgba(0, 0, 0, 0.45);
  outline: none;
  outline-color: transparent;
  box-shadow: none;
  min-width: 10px;
  min-height: 10px;
  margin-top: -4px;
  margin-bottom: -4px;
  margin-left: 0;
  margin-right: 0;
}
scale trough {
  background-color: rgba(255, 255, 255, 0.20);
  background-image: none;
}
scale trough highlight {
  background-color: rgba(255, 255, 255, 0.72);
  background-image: none;
  border-color: transparent;
  box-shadow: none;
}
scale highlight {
  background-color: rgba(255, 255, 255, 0.72);
  background-image: none;
  border-color: transparent;
  box-shadow: none;
}
scale trough fill {
  background-color: rgba(255, 255, 255, 0.72);
  background-image: none;
  border-color: transparent;
  box-shadow: none;
}
scale fill {
  background-color: rgba(255, 255, 255, 0.72);
  background-image: none;
  border-color: transparent;
  box-shadow: none;
}
scale slider {
  background-color: rgba(255, 255, 255, 0.95);
  background-image: none;
  border-color: rgba(0, 0, 0, 0.45);
  outline: none;
  box-shadow: none;
}
"#;

#[repr(C)]
pub struct GdkDisplay {
    _private: [u8; 0],
}

#[repr(C)]
pub struct GtkCssProvider {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn g_free(mem: *mut c_void);
    fn g_object_unref(object: *mut c_void);
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn gdk_display_get_default() -> *mut GdkDisplay;
    fn gtk_css_provider_load_from_string(provider: *mut GtkCssProvider, string: *const c_char);
    fn gtk_css_provider_new() -> *mut GtkCssProvider;
    fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut c_void,
        priority: u32,
    );
}

unsafe fn duplicate_css(css: &str) -> *mut c_char {
    let mut bytes = Vec::with_capacity(css.len() + 1);
    bytes.extend_from_slice(css.as_bytes());
    bytes.push(0);
    g_strdup(bytes.as_ptr() as *const c_char)
}

unsafe fn install_css(css: &str, priority: u32) {
    let mut bytes = Vec::with_capacity(css.len() + 1);
    bytes.extend_from_slice(css.as_bytes());
    bytes.push(0);

    let provider = gtk_css_provider_new();
    gtk_css_provider_load_from_string(provider, bytes.as_ptr() as *const c_char);
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        provider as *mut c_void,
        priority,
    );
    g_object_unref(provider as *mut c_void);
}

unsafe fn player_style_build_overlay_css() -> *mut c_char {
    duplicate_css(OVERLAY_CSS)
}

unsafe fn player_style_build_footer_css() -> *mut c_char {
    duplicate_css(FOOTER_CSS)
}

pub unsafe fn player_style_install_footer_css() {
    static mut INSTALLED: bool = false;

    if INSTALLED {
        return;
    }

    let css = player_style_build_footer_css();
    if css.is_null() {
        return;
    }
    g_free(css as *mut c_void);

    install_css(FOOTER_CSS, GTK_STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    INSTALLED = true;
}

pub unsafe fn player_style_install_overlay_css() {
    static mut INSTALLED: bool = false;

    if INSTALLED {
        return;
    }

    let css = player_style_build_overlay_css();
    if css.is_null() {
        return;
    }
    g_free(css as *mut c_void);

    install_css(OVERLAY_CSS, GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
    INSTALLED = true;
}
