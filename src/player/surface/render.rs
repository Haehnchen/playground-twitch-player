use super::ffi::{
    G_SOURCE_CONTINUE, G_SOURCE_REMOVE, GL_COLOR_BUFFER_BIT, GL_FRAMEBUFFER_BINDING, GdkGLContext,
    GtkGLArea, GtkWidget, MPV_MAINLOOP_PRIORITY, epoxy_eglGetProcAddress, epoxy_glClear,
    epoxy_glClearColor, epoxy_glGetIntegerv, g_atomic_int_compare_and_exchange, g_atomic_int_set,
    gtk_gl_area_attach_buffers, gtk_gl_area_get_error, gtk_gl_area_make_current,
    gtk_gl_area_queue_render, gtk_gl_area_set_auto_render, gtk_widget_get_height,
    gtk_widget_get_realized, gtk_widget_get_scale_factor, gtk_widget_get_width,
};
use super::playback::{set_tile_status, tile_mpv};
use super::runtime::{
    add_tile_idle, add_tile_timeout, log_warning, remove_source_if_active, tile_is_closing,
};
use super::{FALSE, StreamTile, TRUE, player_surface_ref, player_surface_unref};
use crate::player::session::player_session_reenable_video;
use crate::player::session::report;
use libmpv2::Mpv;
// libmpv2 7.0.1 omits internal_format in its OpenGL framebuffer type.
use libmpv2_sys::{
    self as mpv_sys, mpv_opengl_fbo as MpvOpenGLFbo, mpv_opengl_init_params as MpvOpenGLInitParams,
    mpv_render_param as MpvRenderParam,
    mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE as MPV_RENDER_PARAM_API_TYPE,
    mpv_render_param_type_MPV_RENDER_PARAM_FLIP_Y as MPV_RENDER_PARAM_FLIP_Y,
    mpv_render_param_type_MPV_RENDER_PARAM_INVALID as MPV_RENDER_PARAM_INVALID,
    mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO as MPV_RENDER_PARAM_OPENGL_FBO,
    mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS as MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
    mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME as MPV_RENDER_UPDATE_FRAME,
};
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::Ordering;
use std::{ptr::NonNull, rc::Rc};

pub(super) struct PlayerRenderContext {
    ctx: NonNull<mpv_sys::mpv_render_context>,
    _mpv: Rc<Mpv>,
}

impl Drop for PlayerRenderContext {
    fn drop(&mut self) {
        unsafe {
            mpv_sys::mpv_render_context_set_update_callback(
                self.ctx.as_ptr(),
                None,
                ptr::null_mut(),
            );
            mpv_sys::mpv_render_context_free(self.ctx.as_ptr());
        }
    }
}

pub(super) unsafe fn enable_tile_render_callback(tile: *mut StreamTile) {
    unsafe {
        let Some(context) = (*tile).mpv_gl.as_ref() else {
            return;
        };

        if (*tile).render_callback_ref_held == 0 {
            player_surface_ref((*tile).app);
            (*tile).render_callback_ref_held = TRUE;
        }
        mpv_sys::mpv_render_context_set_update_callback(
            context.ctx.as_ptr(),
            Some(on_mpv_render_update),
            tile as *mut c_void,
        );
    }
}

pub(super) unsafe fn disable_tile_render_callback(tile: *mut StreamTile) {
    unsafe {
        if let Some(context) = (*tile).mpv_gl.as_ref() {
            mpv_sys::mpv_render_context_set_update_callback(
                context.ctx.as_ptr(),
                None,
                ptr::null_mut(),
            );
        }
        if (*tile).render_callback_ref_held != 0 {
            (*tile).render_callback_ref_held = FALSE;
            player_surface_unref((*tile).app);
        }
    }
}

pub(super) unsafe extern "C" fn get_proc_address(
    _ctx: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    unsafe { (epoxy_eglGetProcAddress)(name) }
}

pub(super) unsafe fn configure_gl_area(area: *mut GtkGLArea) {
    unsafe {
        gtk_gl_area_set_auto_render(area, FALSE);
    }
}

pub(super) unsafe extern "C" fn queue_mpv_render(user_data: *mut c_void) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;

        g_atomic_int_set(&mut (*tile).render_queued, 0);

        queue_tile_render(tile);

        G_SOURCE_REMOVE
    }
}

pub(super) unsafe fn queue_tile_render(tile: *mut StreamTile) {
    unsafe {
        if !(*(*tile).app).closing.load(Ordering::Acquire) && !(*tile).gl_area.is_null() {
            gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
        }
    }
}

pub(super) unsafe extern "C" fn warmup_tile_render(user_data: *mut c_void) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;

        if (*(*tile).app).closing.load(Ordering::Acquire)
            || (*tile).gl_area.is_null()
            || (*tile).render_warmup_frames <= 0
        {
            (*tile).render_warmup_source = 0;
            return G_SOURCE_REMOVE;
        }

        (*tile).render_warmup_frames -= 1;
        gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
        G_SOURCE_CONTINUE
    }
}

pub(super) unsafe fn start_render_warmup(tile: *mut StreamTile) {
    unsafe {
        remove_source_if_active(&mut (*tile).render_warmup_source);
        (*tile).render_warmup_frames = 90;
        (*tile).render_warmup_source = add_tile_timeout(16, Some(warmup_tile_render), tile);
    }
}

// Coalesce mpv updates into one pending GTK render request.
pub(super) unsafe extern "C" fn on_mpv_render_update(ctx: *mut c_void) {
    unsafe {
        let tile = ctx as *mut StreamTile;
        if tile_is_closing(tile) {
            return;
        }

        if g_atomic_int_compare_and_exchange(&mut (*tile).render_queued, 0, 1) != 0 {
            add_tile_idle(MPV_MAINLOOP_PRIORITY, Some(queue_mpv_render), tile);
        }
    }
}

// Disable render callbacks before freeing the OpenGL context.
pub(super) unsafe fn clear_tile_render_context(tile: *mut StreamTile) {
    unsafe {
        if !(*tile).gl_area.is_null() && gtk_widget_get_realized((*tile).gl_area) != 0 {
            gtk_gl_area_make_current((*tile).gl_area as *mut GtkGLArea);
        }

        if (*tile).mpv_gl.is_some() {
            disable_tile_render_callback(tile);
            (*tile).mpv_gl.take();
        }
        remove_source_if_active(&mut (*tile).render_warmup_source);
        (*tile).last_render_width = 0;
        (*tile).last_render_height = 0;
        (*tile).render_warmup_frames = 0;
    }
}

// Render into the current GtkGLArea framebuffer.
pub(super) unsafe extern "C" fn on_gl_render(
    area: *mut GtkGLArea,
    _context: *mut GdkGLContext,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        let tile = user_data as *mut StreamTile;

        if tile_is_closing(tile) {
            return TRUE;
        }

        if (*tile).mpv_gl.is_none() {
            gtk_gl_area_attach_buffers(area);
            (epoxy_glClearColor)(0.02, 0.02, 0.02, 1.0);
            (epoxy_glClear)(GL_COLOR_BUFFER_BIT);
            return TRUE;
        }

        let scale = gtk_widget_get_scale_factor(area as *mut GtkWidget);
        let width = gtk_widget_get_width(area as *mut GtkWidget) * scale;
        let height = gtk_widget_get_height(area as *mut GtkWidget) * scale;

        if width <= 0 || height <= 0 {
            return TRUE;
        }

        let update_flags =
            mpv_sys::mpv_render_context_update((*tile).mpv_gl.as_ref().unwrap().ctx.as_ptr());
        let size_changed =
            width != (*tile).last_render_width || height != (*tile).last_render_height;
        let warming_up = (*tile).render_warmup_frames > 0;

        if (update_flags & MPV_RENDER_UPDATE_FRAME as u64) == 0 && !size_changed && !warming_up {
            return TRUE;
        }

        gtk_gl_area_attach_buffers(area);

        let mut current_fbo: c_int = 0;
        (epoxy_glGetIntegerv)(GL_FRAMEBUFFER_BINDING, &mut current_fbo);

        let mut fbo = MpvOpenGLFbo {
            fbo: current_fbo,
            w: width,
            h: height,
            internal_format: 0,
        };
        let mut flip_y: c_int = 1;
        let mut params = [
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_OPENGL_FBO,
                data: &mut fbo as *mut MpvOpenGLFbo as *mut c_void,
            },
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_FLIP_Y,
                data: &mut flip_y as *mut c_int as *mut c_void,
            },
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];

        let status = mpv_sys::mpv_render_context_render(
            (*tile).mpv_gl.as_ref().unwrap().ctx.as_ptr(),
            params.as_mut_ptr(),
        );
        if status < 0 {
            report(Err(libmpv2::Error::Raw(status)), "mpv render");
        } else {
            (*tile).last_render_width = width;
            (*tile).last_render_height = height;
        }

        TRUE
    }
}

pub(super) unsafe fn create_mpv_render_context(tile: *mut StreamTile) -> c_int {
    unsafe {
        let Some(mpv) = tile_mpv(tile) else {
            return FALSE;
        };
        if (*tile).gl_area.is_null() {
            return FALSE;
        }

        gtk_gl_area_make_current((*tile).gl_area as *mut GtkGLArea);

        let gl_error = gtk_gl_area_get_error((*tile).gl_area as *mut GtkGLArea);
        if !gl_error.is_null() {
            log_warning(c"GTK GL area error: %s".as_ptr(), (*gl_error).message);
            return FALSE;
        }

        if (*tile).mpv_gl.is_some() {
            disable_tile_render_callback(tile);
            (*tile).mpv_gl.take();
        }

        let mut gl_init_params = MpvOpenGLInitParams {
            get_proc_address: Some(get_proc_address),
            get_proc_address_ctx: ptr::null_mut(),
        };
        let mut params = [
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_API_TYPE,
                data: c"opengl".as_ptr() as *mut c_void,
            },
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: &mut gl_init_params as *mut MpvOpenGLInitParams as *mut c_void,
            },
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];

        let mut ctx = ptr::null_mut();
        let status =
            mpv_sys::mpv_render_context_create(&mut ctx, mpv.ctx.as_ptr(), params.as_mut_ptr());
        if status < 0 {
            report(Err(libmpv2::Error::Raw(status)), "mpv render context");
            return FALSE;
        }

        let Some(ctx) = NonNull::new(ctx) else {
            return FALSE;
        };
        (*tile).mpv_gl = Some(Box::new(PlayerRenderContext {
            ctx,
            _mpv: mpv,
        }));
        enable_tile_render_callback(tile);
        // The session may already be playing; force mpv to bind video to this new GLArea.
        player_session_reenable_video((*tile).session);
        start_render_warmup(tile);
        gtk_gl_area_queue_render((*tile).gl_area as *mut GtkGLArea);
        TRUE
    }
}

pub(super) unsafe extern "C" fn on_gl_realize(_area: *mut GtkGLArea, user_data: *mut c_void) {
    unsafe {
        let tile = user_data as *mut StreamTile;

        if tile_is_closing(tile) {
            return;
        }

        if tile_mpv(tile).is_some() && create_mpv_render_context(tile) == 0 {
            set_tile_status(tile, c"Render error".as_ptr());
        }
    }
}

pub(super) unsafe extern "C" fn on_gl_unrealize(area: *mut GtkGLArea, user_data: *mut c_void) {
    unsafe {
        let tile = user_data as *mut StreamTile;

        if tile_is_closing(tile) {
            return;
        }

        gtk_gl_area_make_current(area);
        clear_tile_render_context(tile);
    }
}
