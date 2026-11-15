//! Preview image loading and decoding.

use super::ffi::*;
use super::runtime::{error_message, g_object_unref_destroy, log_debug, ptr_array_index};
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_ref, channel_switcher_overlay_unref};
use crate::glib_strings::is_nonempty;
use crate::twitch_stream_info::{GAsyncResult, GCancellable, GError, GPtrArray};
use std::ffi::{c_char, c_double, c_int, c_void};
use std::ptr;
use std::sync::atomic::Ordering;

struct RemoteImageData {
    switcher: *mut ChannelSwitcherOverlay,
    url: *mut c_char,
    cache_key: *mut c_char,
    waiters: *mut GPtrArray,
    width: c_int,
    height: c_int,
}

struct DecodeRemoteImageData {
    bytes: *mut GBytes,
    width: c_int,
    height: c_int,
}

struct DecodedRemoteImage {
    bytes: *mut GBytes,
    width: c_int,
    height: c_int,
    stride: usize,
}

unsafe fn remote_image_data_free(data: *mut RemoteImageData) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data);
        g_free(data.url as *mut c_void);
        g_free(data.cache_key as *mut c_void);
        g_ptr_array_unref(data.waiters);
        channel_switcher_overlay_unref(data.switcher);
    }
}

unsafe extern "C" fn decode_remote_image_data_free(data: *mut c_void) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data as *mut DecodeRemoteImageData);
        g_bytes_unref(data.bytes);
    }
}

unsafe extern "C" fn decoded_remote_image_free(data: *mut c_void) {
    unsafe {
        if data.is_null() {
            return;
        }

        let data = Box::from_raw(data as *mut DecodedRemoteImage);
        g_bytes_unref(data.bytes);
    }
}

pub(super) unsafe fn clear_image_cache(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if !(*switcher).image_cache.is_null() {
            g_hash_table_remove_all((*switcher).image_cache);
        }
        if !(*switcher).image_waiters.is_null() {
            g_hash_table_remove_all((*switcher).image_waiters);
        }
    }
}

unsafe fn set_remote_image_texture(image: *mut GtkWidget, texture: *mut GdkTexture) {
    unsafe {
        if image.is_null() || texture.is_null() {
            return;
        }

        gtk_picture_set_paintable(image as *mut GtkPicture, texture as *mut GdkPaintable);
    }
}

unsafe fn build_image_cache_key(url: *const c_char, width: c_int, height: c_int) -> *mut c_char {
    unsafe { g_strdup_printf(c"%s\n%d:%d".as_ptr(), url, width, height) }
}

unsafe fn decode_cover_image_from_bytes(
    bytes: *mut GBytes,
    width: c_int,
    height: c_int,
    error: *mut *mut GError,
) -> *mut DecodedRemoteImage {
    unsafe {
        if width <= 0 || height <= 0 {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_INVALID_ARGUMENT,
                c"invalid image target size".as_ptr(),
            );
            return ptr::null_mut();
        }

        let stream = g_memory_input_stream_new_from_bytes(bytes);
        let source = gdk_pixbuf_new_from_stream(stream, ptr::null_mut(), error);
        g_object_unref(stream as *mut c_void);
        if source.is_null() {
            return ptr::null_mut();
        }

        let source_width = gdk_pixbuf_get_width(source);
        let source_height = gdk_pixbuf_get_height(source);
        if source_width <= 0 || source_height <= 0 {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"invalid image dimensions".as_ptr(),
            );
            g_object_unref(source as *mut c_void);
            return ptr::null_mut();
        }

        let scale = (width as c_double / source_width as c_double)
            .max(height as c_double / source_height as c_double);
        let scaled_width = source_width as c_double * scale;
        let scaled_height = source_height as c_double * scale;
        let offset_x = (width as c_double - scaled_width) / 2.0;
        let offset_y = (height as c_double - scaled_height) / 2.0;

        let target = gdk_pixbuf_new(GDK_COLORSPACE_RGB, TRUE, 8, width, height);
        if target.is_null() {
            g_set_error(
                error,
                g_io_error_quark(),
                G_IO_ERROR_FAILED,
                c"could not allocate image target".as_ptr(),
            );
            g_object_unref(source as *mut c_void);
            return ptr::null_mut();
        }

        gdk_pixbuf_fill(target, 0x00000000);
        gdk_pixbuf_composite(
            source,
            target,
            0,
            0,
            width,
            height,
            offset_x,
            offset_y,
            scale,
            scale,
            GDK_INTERP_BILINEAR,
            255,
        );

        let stride = gdk_pixbuf_get_rowstride(target) as usize;
        let length = stride * height as usize;
        let texture_bytes = g_bytes_new(gdk_pixbuf_get_pixels(target) as *const c_void, length);
        g_object_unref(target as *mut c_void);
        g_object_unref(source as *mut c_void);
        Box::into_raw(Box::new(DecodedRemoteImage {
            bytes: texture_bytes,
            width,
            height,
            stride,
        }))
    }
}

unsafe fn remove_image_waiters_if_current(data: *mut RemoteImageData) {
    unsafe {
        let switcher = (*data).switcher;
        if (*switcher).image_waiters.is_null() {
            return;
        }

        let current = g_hash_table_lookup(
            (*switcher).image_waiters,
            (*data).cache_key as *const c_void,
        ) as *mut GPtrArray;
        if current == (*data).waiters {
            g_hash_table_remove(
                (*switcher).image_waiters,
                (*data).cache_key as *const c_void,
            );
        }
    }
}

// Decode and scale pixels on a worker thread.
unsafe extern "C" fn decode_remote_image_worker(
    task: *mut GTask,
    _source_object: *mut c_void,
    task_data: *mut c_void,
    _cancel: *mut GCancellable,
) {
    unsafe {
        let data = task_data as *mut DecodeRemoteImageData;
        let mut error: *mut GError = ptr::null_mut();
        let image =
            decode_cover_image_from_bytes((*data).bytes, (*data).width, (*data).height, &mut error);

        if !error.is_null() {
            g_task_return_error(task, error);
            return;
        }

        g_task_return_pointer(task, image as *mut c_void, Some(decoded_remote_image_free));
    }
}

// Apply decoded images to GTK widgets on the main thread.
unsafe extern "C" fn on_remote_image_decoded(
    _source: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let data = user_data as *mut RemoteImageData;
        let switcher = (*data).switcher;
        let mut error: *mut GError = ptr::null_mut();
        let image =
            g_task_propagate_pointer(result as *mut GTask, &mut error) as *mut DecodedRemoteImage;

        remove_image_waiters_if_current(data);

        if (*switcher).closing.load(Ordering::Acquire) {
            if !image.is_null() {
                decoded_remote_image_free(image as *mut c_void);
            }
            g_clear_error(&mut error);
            remote_image_data_free(data);
            return;
        }

        if !image.is_null() {
            let texture = gdk_memory_texture_new(
                (*image).width,
                (*image).height,
                GDK_MEMORY_R8G8B8A8,
                (*image).bytes,
                (*image).stride,
            );
            if !texture.is_null() {
                if !(*switcher).image_cache.is_null() {
                    g_hash_table_insert(
                        (*switcher).image_cache,
                        g_strdup((*data).cache_key) as *mut c_void,
                        g_object_ref(texture as *mut c_void),
                    );
                }
                for i in 0..(*(*data).waiters).len {
                    set_remote_image_texture(ptr_array_index((*data).waiters, i), texture);
                }
                g_object_unref(texture as *mut c_void);
            }
            decoded_remote_image_free(image as *mut c_void);
        } else if !error.is_null() {
            log_debug(
                c"image decode failed for %s: %s".as_ptr(),
                (*data).url,
                (*error).message,
            );
        }

        g_clear_error(&mut error);
        remote_image_data_free(data);
    }
}

unsafe extern "C" fn on_remote_image_loaded(
    source: *mut c_void,
    result: *mut GAsyncResult,
    user_data: *mut c_void,
) {
    unsafe {
        let data = user_data as *mut RemoteImageData;
        let mut error: *mut GError = ptr::null_mut();
        let mut contents: *mut c_char = ptr::null_mut();
        let mut length: usize = 0;

        if g_file_load_contents_finish(
            source as *mut GFile,
            result,
            &mut contents,
            &mut length,
            ptr::null_mut(),
            &mut error,
        ) == 0
        {
            log_debug(
                c"image load failed for %s: %s".as_ptr(),
                (*data).url,
                error_message(error),
            );
            remove_image_waiters_if_current(data);
            g_clear_error(&mut error);
            remote_image_data_free(data);
            return;
        }

        if (*(*data).switcher).closing.load(Ordering::Acquire) {
            remove_image_waiters_if_current(data);
            g_free(contents as *mut c_void);
            remote_image_data_free(data);
            return;
        }

        let bytes = g_bytes_new_take(contents as *mut c_void, length);
        let decode_data = Box::into_raw(Box::new(DecodeRemoteImageData {
            bytes,
            width: (*data).width,
            height: (*data).height,
        }));
        let task = g_task_new(
            ptr::null_mut(),
            ptr::null_mut(),
            Some(on_remote_image_decoded),
            data as *mut c_void,
        );
        g_task_set_task_data(
            task,
            decode_data as *mut c_void,
            Some(decode_remote_image_data_free),
        );
        g_task_run_in_thread(task, Some(decode_remote_image_worker));
        g_object_unref(task as *mut c_void);
    }
}

// Reuse cached textures and share downloads between matching requests.
pub(super) unsafe fn load_remote_image(
    switcher: *mut ChannelSwitcherOverlay,
    image: *mut GtkWidget,
    url: *const c_char,
    width: c_int,
    height: c_int,
) {
    unsafe {
        if (*switcher).closing.load(Ordering::Acquire)
            || !is_nonempty(url)
            || (*switcher).image_cache.is_null()
            || (*switcher).image_waiters.is_null()
        {
            return;
        }

        let cache_key = build_image_cache_key(url, width, height);
        let cached = g_hash_table_lookup((*switcher).image_cache, cache_key as *const c_void)
            as *mut GdkTexture;
        if !cached.is_null() {
            set_remote_image_texture(image, cached);
            g_free(cache_key as *mut c_void);
            return;
        }

        let mut waiters = g_hash_table_lookup((*switcher).image_waiters, cache_key as *const c_void)
            as *mut GPtrArray;
        if !waiters.is_null() {
            g_ptr_array_add(waiters, g_object_ref(image as *mut c_void));
            g_free(cache_key as *mut c_void);
            return;
        }

        waiters = g_ptr_array_new_with_free_func(Some(g_object_unref_destroy));
        g_ptr_array_add(waiters, g_object_ref(image as *mut c_void));
        g_hash_table_insert(
            (*switcher).image_waiters,
            g_strdup(cache_key) as *mut c_void,
            waiters as *mut c_void,
        );

        let data = Box::into_raw(Box::new(RemoteImageData {
            switcher: channel_switcher_overlay_ref(switcher),
            url: g_strdup(url),
            cache_key: g_strdup(cache_key),
            waiters: g_ptr_array_ref(waiters),
            width,
            height,
        }));

        let file = g_file_new_for_uri(url);
        g_file_load_contents_async(
            file,
            ptr::null_mut(),
            Some(on_remote_image_loaded),
            data as *mut c_void,
        );
        g_object_unref(file as *mut c_void);
        g_free(cache_key as *mut c_void);
    }
}
