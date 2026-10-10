//! Logs window for a single container.

use crate::format;
use crate::msg::Msg;
use fltk::{
    app::Sender,
    button::{Button, CheckButton},
    enums::Font,
    frame::Frame,
    group::Flex,
    prelude::*,
    text::{TextBuffer, TextDisplay},
    window::Window,
};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

const MAX_LINES: usize = 6000;
const KEEP_LINES: usize = 5000;

pub struct LogsWindow {
    pub id: u32,
    pub container_id: String,
    win: Window,
    buf: TextBuffer,
    display: TextDisplay,
    follow: CheckButton,
    lines: usize,
    /// Stream ID of the initial last-500-lines load.
    pub load_stream: u32,
    /// Stream ID of the follow stream while Follow is on.
    pub follow_stream: Option<u32>,
    /// Socket handle of the follow stream, to stop it.
    handle: Option<UnixStream>,
}

impl LogsWindow {
    pub fn new(
        id: u32,
        container_id: String,
        name: &str,
        load_stream: u32,
        s: Sender<Msg>,
    ) -> Self {
        let mut win = Window::default()
            .with_size(800, 500)
            .with_label(&format::label(&format!("Logs — {name}")));
        win.set_xclass("kubustack");
        win.size_range(400, 200, 0, 0);
        win.set_callback(move |_| s.send(Msg::LogClose { window: id }));

        let mut col = Flex::default_fill().column();
        col.set_margin(4);
        let buf = TextBuffer::default();
        let mut display = TextDisplay::default();
        display.set_buffer(buf.clone());
        display.set_text_font(Font::Courier);
        display.set_text_size(13);

        let mut row = Flex::default().row();
        let mut follow = CheckButton::default().with_label("Follow");
        follow.set_callback(move |b| {
            s.send(Msg::LogFollow {
                window: id,
                on: b.is_checked(),
            })
        });
        row.fixed(&follow, 90);
        Frame::default();
        let mut close = Button::default().with_label("Close");
        close.set_callback(move |_| s.send(Msg::LogClose { window: id }));
        row.fixed(&close, 90);
        row.end();
        col.fixed(&row, 28);
        col.end();

        win.resizable(&col);
        win.end();
        win.show();

        LogsWindow {
            id,
            container_id,
            win,
            buf,
            display,
            follow,
            lines: 0,
            load_stream,
            follow_stream: None,
            handle: None,
        }
    }

    /// Appends text, trims the top past 6000 lines back to 5000, and scrolls to the end.
    pub fn append(&mut self, text: &str) {
        self.buf.append(text);
        self.lines += text.matches('\n').count();
        if self.lines > MAX_LINES {
            let all = self.buf.text();
            let drop = self.lines - KEEP_LINES;
            if let Some((i, _)) = all.match_indices('\n').nth(drop - 1) {
                self.buf.remove(0, i as i32 + 1);
                self.lines -= drop;
            }
        }
        self.display.set_insert_position(self.buf.length());
        self.display.show_insert_position();
    }

    pub fn focus(&mut self) {
        self.win.show();
    }

    /// Keeps the handle of a newly started follow stream, or stops it if no longer wanted.
    pub fn take_handle(&mut self, stream: u32, handle: UnixStream) {
        if self.follow_stream == Some(stream) {
            self.handle = Some(handle);
        } else {
            let _ = handle.shutdown(Shutdown::Both);
        }
    }

    /// Stops the follow stream, if any. Its reader thread then ends.
    pub fn stop_follow(&mut self) {
        self.follow_stream = None;
        if let Some(h) = self.handle.take() {
            let _ = h.shutdown(Shutdown::Both);
        }
    }

    /// The follow stream ended on its own (e.g. the container stopped).
    pub fn follow_ended(&mut self) {
        self.stop_follow();
        self.follow.set_checked(false);
    }

    pub fn close(mut self) {
        self.stop_follow();
        self.win.hide();
        Window::delete(self.win);
    }
}
