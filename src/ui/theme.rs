//! Flat, modern look drawn by FLTK itself: light/dark palette following the desktop's
//! preference, rounded boxes, roomy rows.

use fltk::{
    app, draw,
    enums::{Color, FrameType},
};
use std::process::Command;

// Named colors live in FLTK's free colormap slots (16+), filled by `apply` for the chosen
// palette, so they stay constants and work in browser `@C` format codes.
pub const ACCENT: Color = Color::from_rgbi(16);
pub const MUTED: Color = Color::from_rgbi(17);
pub const GREEN: Color = Color::from_rgbi(18);
pub const RED: Color = Color::from_rgbi(19);
pub const SURFACE: Color = Color::from_rgbi(20);
const BORDER: Color = Color::from_rgbi(21);
/// Selected list rows and pressed buttons: a soft tint that keeps text readable.
pub const SELECTION: Color = Color::from_rgbi(22);
/// Text on accent-filled areas (the selected tab).
pub const ON_ACCENT: Color = Color::from_rgbi(23);
pub const CONSOLE_BG: Color = Color::from_rgb(0x1E, 0x1F, 0x22);
pub const CONSOLE_FG: Color = Color::from_rgb(0xD4, 0xD4, 0xD4);

type Rgb = (u8, u8, u8);

struct Palette {
    background: Rgb,
    surface: Rgb,
    text: Rgb,
    muted: Rgb,
    border: Rgb,
    accent: Rgb,
    selection: Rgb,
    on_accent: Rgb,
    green: Rgb,
    red: Rgb,
}

const LIGHT: Palette = Palette {
    background: (0xF4, 0xF5, 0xF7),
    surface: (0xFF, 0xFF, 0xFF),
    text: (0x1F, 0x23, 0x28),
    muted: (0x6B, 0x72, 0x80),
    border: (0xD5, 0xD9, 0xE0),
    accent: (0x0E, 0x7C, 0x99),
    selection: (0xCF, 0xEC, 0xF4),
    on_accent: (0xFF, 0xFF, 0xFF),
    green: (0x1A, 0x7F, 0x37),
    red: (0xCF, 0x22, 0x2E),
};

const DARK: Palette = Palette {
    background: (0x1B, 0x1E, 0x23),
    surface: (0x24, 0x28, 0x2F),
    text: (0xE3, 0xE6, 0xEA),
    muted: (0x8F, 0x97, 0xA3),
    border: (0x38, 0x3E, 0x48),
    accent: (0x22, 0xB6, 0xCE),
    selection: (0x17, 0x4A, 0x58),
    on_accent: (0x0B, 0x1F, 0x25),
    green: (0x3F, 0xB9, 0x50),
    red: (0xF2, 0x55, 0x5A),
};

/// Prefix of every list line: an empty first column whose large font size makes rows
/// taller (`Fl_Browser` has no line spacing setting). Widths start with [`PAD`].
pub const ROW: &str = "@S24@. \t";
pub const PAD: i32 = 6;

const RADIUS: i32 = 6;

/// Sets colors and replaces the raised/sunken box styles with flat rounded ones.
pub fn apply() {
    let p = if prefers_dark() { &DARK } else { &LIGHT };
    app::set_scheme(app::Scheme::Base);
    let (r, g, b) = p.background;
    app::background(r, g, b);
    let (r, g, b) = p.surface;
    app::background2(r, g, b);
    let (r, g, b) = p.text;
    app::foreground(r, g, b);
    let (r, g, b) = p.accent;
    app::set_selection_color(r, g, b);
    for (slot, (r, g, b)) in [
        (ACCENT, p.accent),
        (MUTED, p.muted),
        (GREEN, p.green),
        (RED, p.red),
        (SURFACE, p.surface),
        (BORDER, p.border),
        (SELECTION, p.selection),
        (ON_ACCENT, p.on_accent),
    ] {
        app::set_color(slot, r, g, b);
    }
    // ponytail: no dotted focus boxes; keyboard use is list-centric (arrows, Enter, Delete,
    // F5) and the selection highlight shows where you are.
    app::set_visible_focus(false);
    app::set_scrollbar_size(12);
    app::set_frame_type_cb(FrameType::UpBox, button, 1, 1, 2, 2);
    app::set_frame_type_cb(FrameType::ThinUpBox, button, 1, 1, 2, 2);
    app::set_frame_type_cb(FrameType::DownBox, pressed, 1, 1, 2, 2);
    app::set_frame_type_cb(FrameType::ThinDownBox, field, 1, 1, 2, 2);
}

/// Dark if `KUBUSTACK_THEME=dark`, else the desktop's preference: `GTK_THEME`, then the
/// freedesktop portal's color scheme (GNOME, KDE, Cinnamon), then a `*dark*` GTK theme name.
/// Read once at startup; restart the app after switching.
fn prefers_dark() -> bool {
    if let Ok(t) = std::env::var("KUBUSTACK_THEME") {
        return t.eq_ignore_ascii_case("dark");
    }
    if std::env::var("GTK_THEME").is_ok_and(|t| dark_name(&t)) {
        return true;
    }
    let portal = output(
        "gdbus",
        &[
            "call",
            "--session",
            "--timeout=1",
            "--dest=org.freedesktop.portal.Desktop",
            "--object-path=/org/freedesktop/portal/desktop",
            "--method=org.freedesktop.portal.Settings.Read",
            "org.freedesktop.appearance",
            "color-scheme",
        ],
    );
    if let Some(dark) = portal.as_deref().and_then(portal_dark) {
        return dark;
    }
    [
        "org.gnome.desktop.interface",
        "org.cinnamon.desktop.interface",
    ]
    .iter()
    .any(|schema| output("gsettings", &["get", schema, "gtk-theme"]).is_some_and(|t| dark_name(&t)))
}

/// Stdout of a successful command, or `None` if it's missing or fails.
fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Portal reply like `(<<uint32 1>>,)`: 1 dark, 2 light, 0 no preference (`None`).
fn portal_dark(reply: &str) -> Option<bool> {
    let n = reply.split("uint32 ").nth(1)?.chars().next()?;
    match n {
        '1' => Some(true),
        '2' => Some(false),
        _ => None,
    }
}

fn dark_name(theme: &str) -> bool {
    theme.to_lowercase().contains("dark")
}

/// Rounded fill with a border: buttons, tabs, scrollbar thumbs.
fn button(x: i32, y: i32, w: i32, h: i32, c: Color) {
    let fill = if c == Color::Background { SURFACE } else { c };
    draw::draw_rbox(x, y, w, h, RADIUS, true, fill);
    draw::draw_rbox(x, y, w, h, RADIUS, false, BORDER);
}

/// Pressed button: same shape, tinted.
fn pressed(x: i32, y: i32, w: i32, h: i32, c: Color) {
    let fill = if c == Color::Background {
        SELECTION
    } else {
        c.darker()
    };
    draw::draw_rbox(x, y, w, h, RADIUS, true, fill);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_replies() {
        assert_eq!(portal_dark("(<<uint32 1>>,)\n"), Some(true));
        assert_eq!(portal_dark("(<uint32 2>,)"), Some(false));
        assert_eq!(portal_dark("(<<uint32 0>>,)"), None);
        assert_eq!(portal_dark("garbage"), None);
    }

    #[test]
    fn theme_names() {
        assert!(dark_name("'Mint-Y-Dark-Aqua'\n"));
        assert!(dark_name("Adwaita:dark"));
        assert!(!dark_name("'Mint-Y-Aqua'"));
    }
}
