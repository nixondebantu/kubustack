//! Main window: tabs, status bar, error panel.

use fltk::{enums::Align, frame::Frame, group, image::SvgImage, prelude::*, window::Window};

const ICON_SVG: &str = include_str!("../../assets/kubustack.svg");

/// Builds and shows the main window.
pub fn show() {
    let mut win = Window::default()
        .with_size(900, 560)
        .with_label("KUBUSTACK");
    win.set_xclass("kubustack");
    win.size_range(640, 400, 0, 0);
    if let Ok(icon) = SvgImage::from_data(ICON_SVG) {
        win.set_icon(Some(icon));
    }

    let mut col = group::Flex::default_fill().column();
    col.set_margin(4);

    let mut tabs = group::Tabs::default_fill();
    group::Group::default_fill().with_label("Containers").end();
    group::Group::default_fill().with_label("Images").end();
    tabs.end();

    let mut status = Frame::default().with_label("Not connected");
    status.set_align(Align::Left | Align::Inside);
    col.fixed(&status, 22);
    col.end();

    win.resizable(&col);
    win.end();
    win.show();
    // Tabs lay out their child groups below the tab headers.
    tabs.auto_layout();
}
