//! Preview loading with decoding on the Gio worker pool.

use super::ffi::GtkWidget;
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_ref, channel_switcher_overlay_unref};
use crate::glib_strings::is_nonempty;
use glib::translate::*;
use gtk4::{gdk, gdk_pixbuf, gio, glib, prelude::*};
use std::{
    cell::RefCell,
    ffi::{CStr, c_char},
    rc::Rc,
    sync::atomic::Ordering,
};

pub(super) type ImageKey = (String, i32, i32);
pub(super) type ImageWaiters = Rc<RefCell<Vec<gtk4::Picture>>>;

struct OverlayRef(*mut ChannelSwitcherOverlay);

impl OverlayRef {
    fn get(&self) -> *mut ChannelSwitcherOverlay {
        self.0
    }
}

impl Drop for OverlayRef {
    fn drop(&mut self) {
        unsafe {
            channel_switcher_overlay_unref(self.0);
        }
    }
}

struct DecodedImage {
    pixels: glib::Bytes,
    stride: usize,
}

pub(super) unsafe fn clear_image_cache(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        (*switcher).image_cache.clear();
        (*switcher).image_waiters.clear();
    }
}

// Scale and crop previews to fill the channel card.
fn decode_cover_image(
    bytes: glib::Bytes,
    width: i32,
    height: i32,
) -> Result<DecodedImage, glib::Error> {
    let invalid = |message| glib::Error::new(gio::IOErrorEnum::InvalidData, message);
    if width <= 0 || height <= 0 {
        return Err(invalid("Invalid image target size"));
    }
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    let source = gdk_pixbuf::Pixbuf::from_stream(&stream, gio::Cancellable::NONE)?;
    let target = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, width, height)
        .ok_or_else(|| invalid("Could not allocate image target"))?;
    let scale = (width as f64 / source.width() as f64).max(height as f64 / source.height() as f64);
    target.fill(0);
    source.composite(
        &target,
        0,
        0,
        width,
        height,
        (width as f64 - source.width() as f64 * scale) / 2.0,
        (height as f64 - source.height() as f64 * scale) / 2.0,
        scale,
        scale,
        gdk_pixbuf::InterpType::Bilinear,
        255,
    );
    Ok(DecodedImage {
        pixels: target.read_pixel_bytes(),
        stride: target.rowstride() as usize,
    })
}

// Download each image once, then update all waiting pictures.
pub(super) unsafe fn load_remote_image(
    switcher: *mut ChannelSwitcherOverlay,
    image: *mut GtkWidget,
    url: *const c_char,
    width: i32,
    height: i32,
) {
    unsafe {
        if (*switcher).closing.load(Ordering::Acquire) || !is_nonempty(url) {
            return;
        }
        let uri = CStr::from_ptr(url).to_string_lossy().into_owned();
        let key = (uri.clone(), width, height);
        let picture: Borrowed<gtk4::Picture> = from_glib_borrow(image.cast());
        if let Some(texture) = (*switcher).image_cache.get(&key) {
            picture.set_paintable(Some(texture));
            return;
        }
        if let Some(waiters) = (*switcher).image_waiters.get(&key) {
            waiters.borrow_mut().push((*picture).clone());
            return;
        }
        let waiters = Rc::new(RefCell::new(vec![(*picture).clone()]));
        (*switcher)
            .image_waiters
            .insert(key.clone(), waiters.clone());
        let owner = OverlayRef(channel_switcher_overlay_ref(switcher));
        glib::MainContext::default().spawn_local(async move {
            let result = async {
                let (bytes, _) = gio::File::for_uri(&uri).load_contents_future().await?;
                gio::spawn_blocking(move || {
                    decode_cover_image(glib::Bytes::from_owned(bytes), width, height)
                })
                .await
                .map_err(|_| glib::Error::new(gio::IOErrorEnum::Failed, "Image worker panicked"))?
            }
            .await;
            let switcher = owner.get();
            if (*switcher)
                .image_waiters
                .get(&key)
                .is_some_and(|current| Rc::ptr_eq(current, &waiters))
            {
                (*switcher).image_waiters.remove(&key);
            }
            if (*switcher).closing.load(Ordering::Acquire) {
                return;
            }
            match result {
                Ok(decoded) => {
                    let texture: gdk::Texture = gdk::MemoryTexture::new(
                        width,
                        height,
                        gdk::MemoryFormat::R8g8b8a8,
                        &decoded.pixels,
                        decoded.stride,
                    )
                    .upcast();
                    (*switcher).image_cache.insert(key, texture.clone());
                    for picture in waiters.borrow().iter() {
                        picture.set_paintable(Some(&texture));
                    }
                }
                Err(err) => glib::g_debug!("twitch-player", "Image load failed for {uri}: {err}"),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_decode_rejects_invalid_sizes_and_image_data() {
        assert!(decode_cover_image(glib::Bytes::from_static(b"invalid"), 32, 18).is_err());
        assert!(decode_cover_image(glib::Bytes::from_static(b""), 0, 18).is_err());
    }

    #[test]
    fn preview_decode_fills_the_requested_rgba_dimensions() {
        let source = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, false, 8, 8, 8).unwrap();
        source.fill(0xff0000ff);
        let png = source.save_to_bufferv("png", &[]).unwrap();
        let image = decode_cover_image(glib::Bytes::from_owned(png), 32, 18).unwrap();
        assert_eq!(image.stride, 32 * 4);
        assert_eq!(image.pixels.len(), 32 * 18 * 4);
        assert_eq!(&image.pixels[..4], &[255, 0, 0, 255]);
    }
}
