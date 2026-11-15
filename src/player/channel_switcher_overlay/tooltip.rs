//! Compact avatar details, with a bounded title instead of the default tooltip.

use gtk4::prelude::*;

pub(super) fn install(button: &gtk4::Button, name: &str, viewers: u32, title: &str) {
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 3);
    content.add_css_class("channel-orbit-tooltip");
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let name = gtk4::Label::builder()
        .label(name)
        .xalign(0.0)
        .valign(gtk4::Align::Baseline)
        .max_width_chars(26)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .css_classes(["channel-orbit-tooltip-name"])
        .build();
    let viewers = gtk4::Label::builder()
        .label(format!("{viewers} viewers"))
        .valign(gtk4::Align::Baseline)
        .css_classes(["channel-orbit-tooltip-viewers"])
        .build();
    header.append(&name);
    header.append(&viewers);
    content.append(&header);
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if !title.is_empty() {
        let title = gtk4::Label::builder()
            .label(title)
            .xalign(0.0)
            .max_width_chars(36)
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .lines(2)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        content.append(&title);
    }
    button.set_has_tooltip(true);
    button.connect_query_tooltip(move |_, _, _, _, tooltip| {
        tooltip.set_custom(Some(&content));
        true
    });
}
