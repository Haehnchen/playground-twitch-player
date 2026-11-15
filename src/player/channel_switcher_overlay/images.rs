//! Preview loading with decoding on the Gio worker pool.

use super::ffi::GtkWidget;
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_ref, channel_switcher_overlay_unref};
use crate::glib_strings::is_nonempty;
use glib::translate::*;
use gtk4::{gdk, gdk_pixbuf, gio, glib, prelude::*};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, c_char},
    rc::Rc,
    sync::atomic::Ordering,
};

pub(super) type ImageKey = (String, i32, i32);
pub(super) type ImageWaiters = Rc<RefCell<Vec<glib::WeakRef<gtk4::Picture>>>>;

type AvatarRequest = (ImageKey, ImageWaiters);

#[derive(Default)]
pub(super) struct AvatarCache {
    textures: HashMap<ImageKey, gdk::Texture>,
    sources: HashMap<String, glib::Bytes>,
    downloads: HashMap<String, Vec<AvatarRequest>>,
    decoding: HashMap<ImageKey, ImageWaiters>,
}

const MAX_CACHED_AVATARS: usize = 256;

fn decode_avatar(
    cache: Rc<RefCell<AvatarCache>>,
    key: ImageKey,
    waiters: ImageWaiters,
    bytes: glib::Bytes,
) {
    glib::MainContext::default().spawn_local(async move {
        let (_, width, height) = key;
        let result = gio::spawn_blocking(move || decode_cover_image(bytes, width, height)).await;
        cache.borrow_mut().decoding.remove(&key);
        match result {
            Ok(Ok(decoded)) => {
                let texture: gdk::Texture = gdk::MemoryTexture::new(
                    width,
                    height,
                    gdk::MemoryFormat::R8g8b8a8,
                    &decoded.pixels,
                    decoded.stride,
                )
                .upcast();
                {
                    let mut cache = cache.borrow_mut();
                    if cache.textures.len() >= MAX_CACHED_AVATARS {
                        cache.textures.clear();
                    }
                    cache.textures.insert(key, texture.clone());
                }
                for picture in waiters.borrow().iter().filter_map(|weak| weak.upgrade()) {
                    picture.set_paintable(Some(&texture));
                }
            }
            _ => glib::g_debug!("twitch-player", "Avatar decode failed for {}", key.0),
        }
    });
}

// Downloads and source bytes are shared even when tiles request different sizes.
fn load_avatar(cache: Rc<RefCell<AvatarCache>>, picture: &gtk4::Picture, key: ImageKey) {
    let mut state = cache.borrow_mut();
    if let Some(texture) = state.textures.get(&key).cloned() {
        drop(state);
        picture.set_paintable(Some(&texture));
        return;
    }
    if let Some(waiters) = state.decoding.get(&key) {
        waiters.borrow_mut().push(picture.downgrade());
        return;
    }
    let waiters = Rc::new(RefCell::new(vec![picture.downgrade()]));
    state.decoding.insert(key.clone(), waiters.clone());
    if let Some(bytes) = state.sources.get(&key.0).cloned() {
        drop(state);
        decode_avatar(cache, key, waiters, bytes);
        return;
    }
    let uri = key.0.clone();
    let downloads = state.downloads.entry(uri.clone()).or_default();
    downloads.push((key, waiters));
    if downloads.len() > 1 {
        return;
    }
    drop(state);
    glib::MainContext::default().spawn_local(async move {
        let result = gio::File::for_uri(&uri).load_contents_future().await;
        let requests = cache
            .borrow_mut()
            .downloads
            .remove(&uri)
            .unwrap_or_default();
        match result {
            Ok((bytes, _)) => {
                let bytes = glib::Bytes::from_owned(bytes);
                {
                    let mut state = cache.borrow_mut();
                    if state.sources.len() >= MAX_CACHED_AVATARS {
                        state.sources.clear();
                    }
                    state.sources.insert(uri, bytes.clone());
                }
                for (key, waiters) in requests {
                    decode_avatar(cache.clone(), key, waiters, bytes.clone());
                }
            }
            Err(error) => {
                for (key, _) in requests {
                    cache.borrow_mut().decoding.remove(&key);
                }
                glib::g_debug!("twitch-player", "Avatar load failed for {uri}: {error}");
            }
        }
    });
}

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
        cancel_image_loads(switcher);
        (*switcher).image_cache.clear();
    }
}

pub(super) unsafe fn cancel_image_loads(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if let Some(cancel) = (*switcher).image_cancel.take() {
            cancel.cancel();
        }
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
        if width == height && width <= 64 {
            load_avatar((*switcher).cache.avatars.clone(), &picture, key);
            return;
        }
        if let Some(texture) = (*switcher).image_cache.get(&key) {
            picture.set_paintable(Some(texture));
            return;
        }
        if let Some(waiters) = (*switcher).image_waiters.get(&key) {
            waiters.borrow_mut().push(picture.downgrade());
            return;
        }
        let waiters = Rc::new(RefCell::new(vec![picture.downgrade()]));
        (*switcher)
            .image_waiters
            .insert(key.clone(), waiters.clone());
        let cancel = (*switcher)
            .image_cancel
            .get_or_insert_with(gio::Cancellable::new)
            .clone();
        let owner = OverlayRef(channel_switcher_overlay_ref(switcher));
        glib::MainContext::default().spawn_local(async move {
            let result = gio::CancellableFuture::new(
                async {
                    let (bytes, _) = gio::File::for_uri(&uri).load_contents_future().await?;
                    gio::spawn_blocking(move || {
                        decode_cover_image(glib::Bytes::from_owned(bytes), width, height)
                    })
                    .await
                    .map_err(|_| {
                        glib::Error::new(gio::IOErrorEnum::Failed, "Image worker panicked")
                    })?
                },
                cancel,
            )
            .await;
            let Ok(result) = result else {
                return;
            };
            let switcher = owner.get();
            if !(*switcher)
                .image_waiters
                .get(&key)
                .is_some_and(|current| Rc::ptr_eq(current, &waiters))
            {
                return;
            }
            (*switcher).image_waiters.remove(&key);
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
                    for picture in waiters.borrow().iter().filter_map(|weak| weak.upgrade()) {
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
