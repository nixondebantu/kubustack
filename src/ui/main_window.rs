//! Main window: tabs, status bar, error panel.

use super::containers_tab::ContainersTab;
use super::images_tab::ImagesTab;
use super::theme;
use crate::format;
use crate::msg::Msg;
use fltk::{
    app::{self, Sender},
    button::Button,
    enums::{Align, Event, Font, FrameType, Key, Shortcut},
    frame::Frame,
    group::{Flex, Tabs},
    image::SvgImage,
    prelude::*,
    window::Window,
};

const ICON_SVG: &str = include_str!("../../assets/kubustack.svg");

pub struct MainWindow {
    col: Flex,
    tabs: Tabs,
    error_panel: Flex,
    error_text: Frame,
    status: Frame,
    pub containers: ContainersTab,
    pub images: ImagesTab,
}

impl MainWindow {
    /// Builds and shows the main window.
    pub fn new(s: Sender<Msg>) -> Self {
        let mut win = Window::default()
            .with_size(900, 560)
            .with_label("KUBUSTACK");
        win.set_xclass("kubustack");
        win.size_range(640, 400, 0, 0);
        if let Ok(icon) = SvgImage::from_data(ICON_SVG) {
            win.set_icon(Some(icon));
        }
        win.handle(move |_, ev| match ev {
            Event::KeyDown | Event::Shortcut if app::event_key() == Key::F5 => {
                s.send(Msg::Refresh);
                true
            }
            Event::KeyDown | Event::Shortcut
                if app::event_state().contains(Shortcut::Ctrl)
                    && app::event_key() == Key::from_char('q') =>
            {
                app::quit();
                true
            }
            Event::KeyDown | Event::Shortcut if app::event_key() == Key::Delete => {
                s.send(Msg::DeleteKey);
                true
            }
            Event::KeyDown | Event::Shortcut
                if matches!(app::event_key(), Key::Enter | Key::KPEnter) =>
            {
                s.send(Msg::EnterKey);
                true
            }
            _ => false,
        });
        win.resize_callback(move |_, _, _, _, _| s.send(Msg::Resized));
        // FLTK closes windows on Escape by default; only quit on a real close.
        win.set_callback(|_| {
            if !(app::event() == Event::Shortcut && app::event_key() == Key::Escape) {
                app::quit();
            }
        });

        let mut col = Flex::default_fill().column();
        col.set_margin(8);

        let mut tabs = Tabs::default_fill();
        // Tabs as pill buttons: the selected one filled with the accent, the other outlined.
        tabs.set_frame(FrameType::UpBox);
        tabs.set_selection_color(theme::ACCENT);
        tabs.set_label_color(theme::ON_ACCENT);
        let containers = ContainersTab::new(s);
        let images = ImagesTab::new(s);
        tabs.end();
        for i in 0..tabs.children() {
            if let Some(mut c) = tabs.child(i) {
                c.set_selection_color(theme::SURFACE);
                c.set_label_color(theme::MUTED);
                c.set_label_font(Font::HelveticaBold);
            }
        }

        let mut error_panel = Flex::default_fill().column();
        error_panel.set_margin(40);
        Frame::default();
        let mut error_text = Frame::default();
        error_text.set_align(Align::Wrap | Align::Inside | Align::Center);
        error_panel.fixed(&error_text, 100);
        let mut retry_row = Flex::default().row();
        Frame::default();
        let mut retry = Button::default().with_label("Retry");
        retry.set_callback(move |_| s.send(Msg::Retry));
        retry_row.fixed(&retry, 100);
        Frame::default();
        retry_row.end();
        error_panel.fixed(&retry_row, 32);
        Frame::default();
        error_panel.end();
        error_panel.hide();

        let mut status = Frame::default();
        status.set_align(Align::Left | Align::Inside);
        status.set_label_color(theme::MUTED);
        status.set_label_size(12);
        col.fixed(&status, 22);
        col.end();

        win.resizable(&col);
        win.end();
        win.show();
        // Tabs lay out their child groups below the tab headers.
        tabs.auto_layout();

        MainWindow {
            col,
            tabs,
            error_panel,
            error_text,
            status,
            containers,
            images,
        }
    }

    pub fn set_status(&mut self, text: &str) {
        self.status.set_label(&format::label(text));
    }

    /// Replaces the tabs with the error panel (disconnected state).
    pub fn show_error(&mut self, message: &str) {
        self.error_text.set_label(&format::label(message));
        self.tabs.hide();
        self.error_panel.show();
        self.col.layout();
    }

    /// Label of the visible tab (labels are space-padded), or `None` while the error panel shows.
    pub fn active_tab(&self) -> Option<String> {
        self.tabs
            .visible()
            .then(|| self.tabs.value())?
            .map(|g| g.label().trim().to_string())
    }

    pub fn show_tabs(&mut self) {
        self.error_panel.hide();
        self.tabs.show();
        self.col.layout();
    }
}
