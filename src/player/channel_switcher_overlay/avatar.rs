//! Draw avatar rings as single paths, avoiding GL seams in rounded CSS borders.

use gtk4::prelude::*;

pub(super) fn frame(
    button: &gtk4::Button,
    picture: &gtk4::Overlay,
    diameter: i32,
) -> gtk4::Overlay {
    picture.set_margin_start(2);
    picture.set_margin_end(2);
    picture.set_margin_top(2);
    picture.set_margin_bottom(2);
    let frame = gtk4::Overlay::new();
    frame.set_child(Some(picture));
    frame.set_overflow(gtk4::Overflow::Visible);
    button.set_overflow(gtk4::Overflow::Visible);

    // The overlay does not affect measurement. Extra space contains the halo
    // and its antialiasing without changing the button's size or hit area.
    let ring = gtk4::DrawingArea::builder()
        .width_request(diameter + 12)
        .height_request(diameter + 12)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .can_target(false)
        .build();
    let weak_button = button.downgrade();
    ring.set_draw_func(move |_, cr, width, height| {
        let Some(button) = weak_button.upgrade() else {
            return;
        };
        let highlighted = button
            .state_flags()
            .intersects(gtk4::StateFlags::PRELIGHT | gtk4::StateFlags::FOCUS_VISIBLE);
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        if highlighted {
            cr.set_source_rgba(145.0 / 255.0, 70.0 / 255.0, 1.0, 0.22);
            cr.set_line_width(4.0);
            cr.arc(
                cx,
                cy,
                diameter as f64 / 2.0 + 2.0,
                0.0,
                std::f64::consts::TAU,
            );
            let _ = cr.stroke();
            cr.set_source_rgb(187.0 / 255.0, 138.0 / 255.0, 1.0);
        } else {
            cr.set_source_rgba(185.0 / 255.0, 159.0 / 255.0, 224.0 / 255.0, 0.55);
        }
        cr.set_line_width(2.0);
        cr.arc(
            cx,
            cy,
            diameter as f64 / 2.0 - 1.0,
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.stroke();
    });
    let weak_ring = ring.downgrade();
    button.connect_state_flags_changed(move |_, _| {
        if let Some(ring) = weak_ring.upgrade() {
            ring.queue_draw();
        }
    });
    frame.add_overlay(&ring);
    frame
}
