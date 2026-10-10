//! User interface built with FLTK. Widgets are only touched on the main thread.

pub mod containers_tab;
pub mod dialogs;
pub mod images_tab;
pub mod logs_window;
pub mod main_window;
pub mod theme;

use crate::format;
use fltk::{
    browser::{Browser, BrowserScrollbar, HoldBrowser},
    draw,
    enums::{Color, Font, FrameType},
    prelude::*,
};

/// Space FLTK leaves between browser columns.
const COLUMN_GAP: i32 = 8;

/// Column widths for a list `total` pixels wide: each fraction of the width (minus the
/// scrollbar), at least 40 px. The last column takes the rest, so it has no fraction.
pub fn column_widths(total: i32, fractions: &[f32]) -> Vec<i32> {
    let usable = (total - 20).max(0) as f32;
    fractions
        .iter()
        .map(|f| ((usable * f) as i32).max(40))
        .collect()
}

/// Non-selectable bold header line above a list. `titles` are tab-separated.
pub fn header(columns: &[i32], titles: &[&str]) -> Browser {
    let mut h = Browser::default();
    h.set_column_widths(&theme::with_pad(columns));
    h.set_has_scrollbar(BrowserScrollbar::None);
    h.set_frame(FrameType::FlatBox);
    h.set_color(Color::Background);
    h.set_text_size(12);
    h.clear_visible_focus();
    let line: Vec<String> = titles
        .iter()
        .map(|t| format!("@b@C{}@.{t}", theme::MUTED.bits()))
        .collect();
    h.add(&format!(" \t{}", line.join("\t")));
    h
}

/// Styles a list: surface color, thin border, soft selection tint.
pub fn style_list(list: &mut HoldBrowser, columns: &[i32]) {
    list.set_frame(FrameType::ThinDownBox);
    list.set_color(theme::SURFACE);
    list.set_selection_color(theme::SELECTION);
    list.set_column_widths(&theme::with_pad(columns));
}

/// Sets the column widths of a list and its header.
pub fn set_columns(header: &mut Browser, list: &mut HoldBrowser, columns: &[i32]) {
    let padded = theme::with_pad(columns);
    header.set_column_widths(&padded);
    header.redraw();
    list.set_column_widths(&padded);
}

/// Replaces a list's lines, keeping its scroll position. Shows a grey `empty` line when there
/// are none. Lines must already be fitted with [`fit`].
pub fn fill(list: &mut HoldBrowser, lines: &[String], empty: &str) {
    let scroll = list.position();
    list.clear();
    if lines.is_empty() {
        list.add(&format!("{}{empty}", theme::ROW));
        list.deactivate();
    } else {
        list.activate();
        for l in lines {
            list.add(&format!("{}{l}", theme::ROW));
        }
    }
    list.set_position(scroll);
    // FLTK doesn't clamp the position; if the list shrank below it, show the last lines.
    let n = list.size();
    if scroll > 0 && !(1..=n).any(|line| list.displayed(line)) {
        list.bottom_line(n);
    }
}

/// Prepares measuring for [`fit`]; call once before fitting a batch of cells.
pub fn measure_with(list: &HoldBrowser) {
    draw::set_font(Font::Helvetica, list.text_size());
}

/// Shortens `text` with `…` until it fits a column `px` wide.
pub fn fit(text: &str, px: i32) -> String {
    let px = f64::from(px - COLUMN_GAP);
    let mut n = text.chars().count();
    let mut out = text.to_string();
    // ponytail: shrinks one char at a time; fine for cell-sized strings.
    while n > 1 && draw::width(&out) > px {
        n -= 1;
        out = format::truncate(text, n);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn widths_scale_with_minimum() {
        assert_eq!(super::column_widths(1020, &[0.5, 0.25]), vec![500, 250]);
        assert_eq!(super::column_widths(100, &[0.5, 0.25]), vec![40, 40]);
    }
}
