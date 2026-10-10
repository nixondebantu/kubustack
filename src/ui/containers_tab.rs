//! Containers tab: list and actions.

use crate::docker::models::ContainerSummary;
use crate::format;
use crate::msg::Msg;
use fltk::{
    app::Sender,
    browser::{Browser, BrowserScrollbar, HoldBrowser},
    button::Button,
    draw,
    enums::{Color, Font, FrameType},
    frame::Frame,
    group::Flex,
    prelude::*,
};

/// Pixel widths of all columns but the last (Created).
const COLUMNS: [i32; 4] = [180, 220, 170, 200];
/// Space FLTK leaves between columns.
const COLUMN_GAP: i32 = 8;

pub struct ContainersTab {
    list: HoldBrowser,
    rows: Vec<ContainerSummary>,
    selected: Option<String>,
}

impl ContainersTab {
    /// Builds the tab inside the current group (the `Tabs`).
    pub fn new(s: Sender<Msg>) -> Self {
        let mut col = Flex::default_fill().column().with_label("Containers");
        col.set_margin(4);

        let mut buttons = Flex::default().row();
        Frame::default();
        let mut refresh = Button::default().with_label("Refresh");
        refresh.set_callback(move |_| s.send(Msg::Refresh));
        buttons.fixed(&refresh, 90);
        buttons.end();
        col.fixed(&buttons, 28);

        let mut header = Browser::default();
        header.set_column_widths(&COLUMNS);
        header.set_has_scrollbar(BrowserScrollbar::None);
        header.set_frame(FrameType::FlatBox);
        header.set_color(Color::Background);
        header.clear_visible_focus();
        header.add("@b@.NAME\t@b@.IMAGE\t@b@.STATUS\t@b@.PORTS\t@b@.CREATED");
        col.fixed(&header, 22);

        let mut list = HoldBrowser::default();
        list.set_column_widths(&COLUMNS);
        list.set_callback(move |_| s.send(Msg::ContainerSelected));
        col.end();

        ContainersTab {
            list,
            rows: Vec::new(),
            selected: None,
        }
    }

    /// Replaces the list, keeping the selection (by ID) and scroll position.
    pub fn set_containers(&mut self, mut rows: Vec<ContainerSummary>) {
        sort(&mut rows);
        let scroll = self.list.position();
        self.list.clear();
        if rows.is_empty() {
            self.list.add("No containers");
            self.list.deactivate();
        } else {
            self.list.activate();
            let now = format::now();
            draw::set_font(Font::Helvetica, self.list.text_size());
            for c in &rows {
                self.list
                    .add(&row(c, now, &|text, i| fit(text, COLUMNS[i])));
            }
        }
        self.rows = rows;
        match self.index_of_selected() {
            Some(i) => self.list.select(i as i32 + 1),
            None => self.selected = None,
        }
        self.list.set_position(scroll);
    }

    /// Records the newly selected container after a click.
    pub fn on_select(&mut self) {
        let line = self.list.value();
        self.selected = (line > 0)
            .then(|| self.rows.get(line as usize - 1))
            .flatten()
            .map(|c| c.id.clone());
    }

    pub fn counts(&self) -> (usize, usize) {
        let running = self.rows.iter().filter(|c| c.state == "running").count();
        (self.rows.len(), running)
    }

    fn index_of_selected(&self) -> Option<usize> {
        let id = self.selected.as_ref()?;
        self.rows.iter().position(|c| &c.id == id)
    }
}

/// Running containers first, then by name.
fn sort(rows: &mut [ContainerSummary]) {
    rows.sort_by_cached_key(|c| {
        (
            c.state != "running",
            format::container_name(&c.names, &c.id).to_lowercase(),
        )
    });
}

/// Shortens `text` with `…` until it fits `px` in the current draw font.
fn fit(text: &str, px: i32) -> String {
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

/// One browser line; `cell(text, column)` fits text to its column.
/// `@.` stops format parsing so cell text is shown literally.
fn row(c: &ContainerSummary, now: i64, cell: &dyn Fn(&str, usize) -> String) -> String {
    let status_color = if c.state == "running" {
        format!("@C{}", Color::DarkGreen.bits())
    } else {
        String::new()
    };
    format!(
        "@.{}\t@.{}\t{status_color}@.{}\t@.{}\t@.{}",
        cell(&format::container_name(&c.names, &c.id), 0),
        cell(&c.image, 1),
        cell(&c.status, 2),
        cell(&format::ports(&c.ports), 3),
        format::relative_time(c.created, now),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(name: &str, state: &str) -> ContainerSummary {
        ContainerSummary {
            id: name.to_string(),
            names: vec![format!("/{name}")],
            image: "img".into(),
            state: state.into(),
            status: "s".into(),
            created: 0,
            ports: vec![],
        }
    }

    #[test]
    fn running_first_then_name() {
        let mut v = vec![
            c("b", "exited"),
            c("Z", "running"),
            c("a", "exited"),
            c("c", "running"),
        ];
        sort(&mut v);
        let names: Vec<_> = v.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(names, ["c", "Z", "a", "b"]);
    }

    #[test]
    fn running_status_is_green() {
        let plain = |t: &str, _: usize| t.to_string();
        let green = format!("@C{}@.s", Color::DarkGreen.bits());
        assert!(row(&c("web", "running"), 0, &plain).contains(&green));
        assert!(!row(&c("web", "exited"), 0, &plain).contains("@C"));
    }
}
