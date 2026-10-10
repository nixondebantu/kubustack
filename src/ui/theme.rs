//! Flat, modern look drawn by FLTK itself: palette, rounded boxes, roomy rows.

use fltk::{
    app, draw,
    enums::{Color, FrameType},
};

pub const ACCENT: Color = Color::from_rgb(0x2F, 0x6F, 0xEB);
pub const MUTED: Color = Color::from_rgb(0x6B, 0x72, 0x80);
pub const GREEN: Color = Color::from_rgb(0x1A, 0x7F, 0x37);
pub const RED: Color = Color::from_rgb(0xCF, 0x22, 0x2E);
pub const SURFACE: Color = Color::from_rgb(0xFF, 0xFF, 0xFF);
const BORDER: Color = Color::from_rgb(0xD5, 0xD9, 0xE0);
pub const CONSOLE_BG: Color = Color::from_rgb(0x1E, 0x1F, 0x22);
pub const CONSOLE_FG: Color = Color::from_rgb(0xD4, 0xD4, 0xD4);

/// Prefix of every list line: an empty first column whose large font size makes rows
/// taller (`Fl_Browser` has no line spacing setting). Widths start with [`PAD`].
pub const ROW: &str = "@S24@. \t";
pub const PAD: i32 = 6;

const RADIUS: i32 = 6;

/// Sets colors and replaces the raised/sunken box styles with flat rounded ones.
pub fn apply() {
    app::set_scheme(app::Scheme::Base);
    app::background(0xF4, 0xF5, 0xF7);
    app::background2(0xFF, 0xFF, 0xFF);
    app::foreground(0x1F, 0x23, 0x28);
    app::set_selection_color(0x2F, 0x6F, 0xEB);
    // ponytail: no dotted focus boxes; keyboard use is list-centric (arrows, Enter, Delete,
    // F5) and the selection highlight shows where you are.
    app::set_visible_focus(false);
    app::set_scrollbar_size(12);
    app::set_frame_type_cb(FrameType::UpBox, button, 1, 1, 2, 2);
    app::set_frame_type_cb(FrameType::ThinUpBox, button, 1, 1, 2, 2);
    app::set_frame_type_cb(FrameType::DownBox, pressed, 1, 1, 2, 2);
    app::set_frame_type_cb(FrameType::ThinDownBox, field, 1, 1, 2, 2);
}

/// Rounded fill with a light border: buttons, tabs, scrollbar thumbs.
fn button(x: i32, y: i32, w: i32, h: i32, c: Color) {
    let fill = if c == Color::Background { SURFACE } else { c };
    draw::draw_rbox(x, y, w, h, RADIUS, true, fill);
    draw::draw_rbox(x, y, w, h, RADIUS, false, BORDER);
}

/// Pressed button: same shape, slightly darker.
fn pressed(x: i32, y: i32, w: i32, h: i32, c: Color) {
    draw::draw_rbox(x, y, w, h, RADIUS, true, c.darker());
    draw::draw_rbox(x, y, w, h, RADIUS, false, BORDER);
}

/// Lists and text areas: square fill with a 1 px border.
fn field(x: i32, y: i32, w: i32, h: i32, c: Color) {
    draw::draw_rect_fill(x, y, w, h, c);
    draw::set_draw_color(BORDER);
    draw::draw_rect(x, y, w, h);
}

/// Browser column widths with the row padding column in front.
pub fn with_pad(widths: &[i32]) -> Vec<i32> {
    std::iter::once(PAD).chain(widths.iter().copied()).collect()
}
