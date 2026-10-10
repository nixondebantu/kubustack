//! User interface built with FLTK. Widgets are only touched on the main thread.

pub mod containers_tab;
pub mod dialogs;
pub mod images_tab;
pub mod logs_window;
pub mod main_window;

use crate::format;
use fltk::{
    browser::{Browser, BrowserScrollbar, HoldBrowser},
    draw,
    enums::{Color, Font, FrameType},
    prelude::*,
};

/// Space FLTK leaves between browser columns.
const COLUMN_GAP: i32 = 8;

/// Non-selectable bold header line above a list. `titles` are tab-separated.
pub fn header(columns: &[i32], titles: &[&str]) -> Browser {
    let mut h = Browser::default();
    h.set_column_widths(columns);
    h.set_has_scrollbar(BrowserScrollbar::None);
    h.set_frame(FrameType::FlatBox);
    h.set_color(Color::Background);
    h.clear_visible_focus();
    let line: Vec<String> = titles.iter().map(|t| format!("@b@.{t}")).collect();
    h.add(&line.join("\t"));
    h
}

/// Replaces a list's lines, keeping its scroll position. Shows a grey `empty` line when there
/// are none. Lines must already be fitted with [`fit`].
pub fn fill(list: &mut HoldBrowser, lines: &[String], empty: &str) {
    let scroll = list.position();
    list.clear();
    if lines.is_empty() {
        list.add(empty);
        list.deactivate();
    } else {
        list.activate();
        for l in lines {
            list.add(l);
        }
    }
    list.set_position(scroll);
    // FLTK doesn't clamp the position; if the list shrank below it, show the last lines.
    let n = list.size();
    if !(1..=n).any(|line| list.displayed(line)) {
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
