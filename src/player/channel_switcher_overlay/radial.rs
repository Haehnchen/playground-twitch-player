//! Proximity-triggered quick selection, sharing the full picker's live/image caches.

use super::actions::{find_settings_channel, on_channel_button_clicked};
use super::channels::fetch_channel_list;
use super::ffi::{GObject, GtkWidget, g_object_set_data_full, g_strdup};
use super::images::load_remote_image;
use super::runtime::{connect_switcher_signal, g_free_destroy, switcher_is_closing};
use super::{ChannelSwitcherOverlay, channel_switcher_overlay_is_visible};
use gtk4::{glib::translate::*, prelude::*};
use std::ffi::{CStr, c_char, c_void};

pub(crate) const PREFETCH_RADIUS: f64 = 150.0;
const OPEN_RADIUS: f64 = 96.0;
const MAX_CHANNELS: usize = 12;
const DEFAULT_SIZE: i32 = 360;
const INNER_RING_CAPACITY: usize = 8;
const INNER_RADIUS: f64 = 0.22;
const OUTER_RADIUS: f64 = 0.36;

pub(super) struct RadialPicker {
    host: gtk4::glib::WeakRef<gtk4::Overlay>,
    fixed: gtk4::Fixed,
    status: gtk4::Label,
    size: i32,
    requested: bool,
    populated: bool,
}

#[derive(Debug)]
struct Slot {
    x: f64,
    y: f64,
}

// Fill the inner ring with up to eight channels before adding the outer ring.
// Each ring distributes its channels evenly, clockwise from twelve o'clock.
fn slots(count: usize, size: i32) -> Vec<Slot> {
    let count = count.min(MAX_CHANNELS);
    let inner = count.min(INNER_RING_CAPACITY);
    let outer = count - inner;
    (0..count)
        .map(|index| {
            let (index, capacity, radius) = if index < inner {
                (index, inner, size as f64 * INNER_RADIUS)
            } else {
                (index - inner, outer, size as f64 * OUTER_RADIUS)
            };
            let angle = index as f64 * std::f64::consts::TAU / capacity as f64;
            Slot {
                x: size as f64 / 2.0 + angle.sin() * radius,
                y: size as f64 / 2.0 - angle.cos() * radius,
            }
        })
        .collect()
}

fn button_size(size: i32) -> i32 {
    (size as f64 * 0.1167).floor() as i32
}

fn in_activation_area(distance: f64, visible: bool, size: i32) -> bool {
    distance
        <= if visible {
            size as f64 * 0.5 + 24.0
        } else {
            OPEN_RADIUS
        }
}

unsafe fn ensure_picker(switcher: *mut ChannelSwitcherOverlay, host: *mut GtkWidget, size: i32) {
    unsafe {
        if (*switcher).radial.is_some() {
            return;
        }
        let host: gtk4::Overlay = from_glib_none(host.cast());
        let fixed = gtk4::Fixed::builder()
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .width_request(size)
            .height_request(size)
            .visible(false)
            .build();
        let status = gtk4::Label::builder()
            .label("Loading live channels…")
            .width_request(size.min(240))
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .can_target(false)
            .css_classes(["channel-orbit-status"])
            .build();
        fixed.put(
            &status,
            ((size - size.min(240)) / 2) as f64,
            size as f64 / 2.0 + 48.0,
        );
        host.add_overlay(&fixed);
        // Keep the existing + button above the fixed container's empty center.
        let mut child = host.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if widget.has_css_class("empty-stream-button") {
                host.remove_overlay(&widget);
                host.add_overlay(&widget);
                break;
            }
        }
        (*switcher).radial = Some(RadialPicker {
            host: host.downgrade(),
            fixed,
            status,
            size,
            requested: false,
            populated: false,
        });
    }
}

/// GTK main thread; host is this tile's overlay and outlives the picker.
pub(crate) unsafe fn channel_switcher_overlay_near_center(
    switcher: *mut ChannelSwitcherOverlay,
    host: *mut GtkWidget,
    x: f64,
    y: f64,
) {
    unsafe {
        if switcher_is_closing(switcher) || channel_switcher_overlay_is_visible(switcher) != 0 {
            return;
        }
        let host_widget: gtk4::Widget = from_glib_none(host.cast());
        let size = DEFAULT_SIZE
            .min(host_widget.width().min(host_widget.height()) - 24)
            .max(1);
        if size < 300 {
            host_widget.add_css_class("compact-channel-orbit");
        } else {
            host_widget.remove_css_class("compact-channel-orbit");
        }
        let distance =
            (x - host_widget.width() as f64 / 2.0).hypot(y - host_widget.height() as f64 / 2.0);
        if (*switcher).radial.is_none() && distance > PREFETCH_RADIUS {
            return;
        }
        ensure_picker(switcher, host, size);
        let radial = (*switcher).radial.as_mut().unwrap();
        if radial.size != size {
            radial.size = size;
            radial.fixed.set_size_request(size, size);
            reset(switcher);
        }
        let radial = (*switcher).radial.as_mut().unwrap();
        radial.requested = in_activation_area(distance, radial.requested, size);
        radial.fixed.set_visible(radial.requested);
        if distance <= PREFETCH_RADIUS || radial.requested {
            render(switcher);
        }
        if distance <= PREFETCH_RADIUS {
            fetch_channel_list(switcher);
        }
    }
}

pub(crate) unsafe fn channel_switcher_overlay_leave_center(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if !switcher_is_closing(switcher) {
            hide(switcher);
        }
    }
}

pub(super) unsafe fn hide(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if let Some(radial) = &mut (*switcher).radial {
            radial.requested = false;
            radial.fixed.set_visible(false);
        }
    }
}

pub(super) unsafe fn reset(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if let Some(radial) = &mut (*switcher).radial {
            while let Some(child) = radial.fixed.first_child() {
                radial.fixed.remove(&child);
            }
            radial.status.set_label("Loading live channels…");
            radial.status.set_visible(true);
            let width = radial.size.min(240);
            radial.status.set_width_request(width);
            radial.fixed.put(
                &radial.status,
                ((radial.size - width) / 2) as f64,
                radial.size as f64 / 2.0 + 48.0,
            );
            radial.populated = false;
        }
    }
}

pub(super) unsafe fn detach(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        if let Some(radial) = (*switcher).radial.take() {
            if let Some(host) = radial.host.upgrade() {
                host.remove_overlay(&radial.fixed);
            }
        }
    }
}

pub(super) unsafe fn show_status(switcher: *mut ChannelSwitcherOverlay, message: *const c_char) {
    unsafe {
        if let Some(radial) = &(*switcher).radial {
            if !radial.populated {
                radial
                    .status
                    .set_label(&CStr::from_ptr(message).to_string_lossy());
            }
        }
    }
}

pub(super) unsafe fn render(switcher: *mut ChannelSwitcherOverlay) {
    unsafe {
        let Some(radial) = &(*switcher).radial else {
            return;
        };
        if radial.populated {
            return;
        }
        let Some(previews) = &(*switcher).previews else {
            return;
        };
        if previews.is_empty() {
            radial
                .status
                .set_label("No channels are live. Click + to choose a channel.");
            return;
        }
        let size = radial.size;
        let fixed = radial.fixed.clone();
        let diameter = button_size(size);
        let image_size = (diameter - 4).max(1);
        // The API already sorts globally; sort indices as well so cached or
        // injected results have the same deterministic ranking.
        let mut ranked: Vec<_> = (0..previews.len()).collect();
        ranked.sort_by(|&a, &b| {
            previews[b]
                .viewer_count
                .cmp(&previews[a].viewer_count)
                .then_with(|| previews[a].channel.cmp(&previews[b].channel))
        });
        ranked.truncate(MAX_CHANNELS);
        let positions = slots(ranked.len(), size);
        let ring_count = if ranked.len() > INNER_RING_CAPACITY {
            2
        } else {
            1
        };
        let guides = gtk4::DrawingArea::builder()
            .width_request(size)
            .height_request(size)
            .can_target(false)
            .build();
        guides.set_draw_func(move |_, cr, _, _| {
            cr.set_line_width(1.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.09);
            for radius in [INNER_RADIUS, OUTER_RADIUS].iter().take(ring_count) {
                cr.arc(
                    size as f64 / 2.0,
                    size as f64 / 2.0,
                    size as f64 * radius,
                    0.0,
                    std::f64::consts::TAU,
                );
                let _ = cr.stroke();
            }
        });
        fixed.put(&guides, 0.0, 0.0);
        for (index, position) in ranked.into_iter().zip(positions) {
            let preview = &(*switcher).previews.as_ref().unwrap()[index];
            let configured = find_settings_channel(switcher, preview.channel.as_ptr());
            let label = if configured.is_null() || (*configured).label.is_empty() {
                &preview.display_name
            } else {
                &(*configured).label
            };
            let name = label.to_string_lossy();
            let button = gtk4::Button::builder()
                .width_request(diameter)
                .height_request(diameter)
                .css_classes(["channel-orbit-avatar"])
                .build();
            super::tooltip::install(
                &button,
                &name,
                preview.viewer_count,
                &preview.title.to_string_lossy(),
            );
            button.update_property(&[gtk4::accessible::Property::Label(&format!(
                "Watch {name}, {} viewers",
                preview.viewer_count
            ))]);
            let avatar = gtk4::Overlay::new();
            let initial = gtk4::Label::new(
                name.chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .as_deref(),
            );
            initial.add_css_class("channel-orbit-initial");
            initial.set_size_request(image_size, image_size);
            avatar.set_child(Some(&initial));
            let picture = gtk4::Picture::builder()
                .width_request(image_size)
                .height_request(image_size)
                .content_fit(gtk4::ContentFit::Cover)
                .can_shrink(true)
                .build();
            avatar.add_overlay(&picture);
            let fallback = initial.downgrade();
            picture.connect_paintable_notify(move |picture| {
                if let Some(initial) = fallback.upgrade() {
                    initial.set_opacity(if picture.paintable().is_some() {
                        0.0
                    } else {
                        1.0
                    });
                }
            });
            avatar.set_overflow(gtk4::Overflow::Hidden);
            avatar.add_css_class("channel-orbit-picture");
            button.set_child(Some(&super::avatar::frame(&button, &avatar, diameter)));
            for (key, value) in [
                (c"channel-name", &preview.channel),
                (c"channel-label", label),
            ] {
                g_object_set_data_full(
                    button.as_ptr().cast::<GObject>(),
                    key.as_ptr(),
                    g_strdup(value.as_ptr()).cast(),
                    Some(g_free_destroy),
                );
            }
            connect_switcher_signal(
                button.as_ptr().cast(),
                c"clicked".as_ptr(),
                on_channel_button_clicked as *const c_void,
                switcher,
            );
            load_remote_image(
                switcher,
                picture.as_ptr().cast(),
                preview.avatar_url.as_ptr(),
                image_size,
                image_size,
            );
            fixed.put(
                &button,
                position.x - diameter as f64 / 2.0,
                position.y - diameter as f64 / 2.0,
            );
        }
        let radial = (*switcher).radial.as_mut().unwrap();
        radial.status.set_visible(false);
        radial.populated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_fill_clockwise_from_top_and_inside_out() {
        for count in [1, 4, 5, 6, 7, 8, 9, 12, 40] {
            let positions = slots(count, DEFAULT_SIZE);
            assert_eq!(positions.len(), count.min(12));
            assert_eq!(positions[0].x, 180.0);
            assert!(positions[0].y < 180.0);
            if count >= 4 {
                assert!(positions[1].x > 180.0);
            }
            if count > 8 {
                assert!(positions[8].y < positions[0].y);
                assert_eq!(positions[8].x, 180.0);
            }
        }
    }

    #[test]
    fn avatars_fit_without_overlap_at_all_preset_sizes() {
        for size in [180, 240, 360] {
            for count in 1..=12 {
                let positions = slots(count, size);
                let diameter = button_size(size) as f64;
                for (i, a) in positions.iter().enumerate() {
                    for coordinate in [a.x, a.y] {
                        assert!(coordinate >= diameter / 2.0);
                        assert!(coordinate + diameter / 2.0 <= size as f64);
                    }
                    for b in positions.iter().skip(i + 1) {
                        assert!((a.x - b.x).hypot(a.y - b.y) > diameter + 2.0);
                    }
                }
            }
        }
    }

    #[test]
    fn activation_keeps_the_route_to_both_rings_open() {
        assert!(PREFETCH_RADIUS > OPEN_RADIUS);
        assert!(!in_activation_area(120.0, false, 360));
        assert!(in_activation_area(90.0, false, 360));
        assert!(in_activation_area(180.0, true, 360));
        assert!(!in_activation_area(220.0, true, 360));
    }

    #[test]
    fn intermediate_counts_use_all_the_space_in_each_ring() {
        for count in 2..=12 {
            let positions = slots(count, 360);
            let capacity = if count <= 8 { count } else { count - 8 };
            let first = if count <= 8 { 0 } else { 8 };
            let radius = if count <= 8 {
                360.0 * 0.22
            } else {
                360.0 * 0.36
            };
            for (i, point) in positions.iter().skip(first).enumerate() {
                let angle = i as f64 * std::f64::consts::TAU / capacity as f64;
                assert!((point.x - (180.0 + angle.sin() * radius)).abs() < 0.01);
                assert!((point.y - (180.0 - angle.cos() * radius)).abs() < 0.01);
            }
        }
    }
}
