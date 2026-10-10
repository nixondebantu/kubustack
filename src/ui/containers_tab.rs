//! Containers tab: list and actions.

use crate::docker::models::ContainerSummary;
use crate::format;
use crate::msg::{Action, Msg};
use fltk::{
    app::Sender, browser::HoldBrowser, button::Button, enums::Color, frame::Frame, group::Flex,
    prelude::*,
};

/// Pixel widths of all columns but the last (Created).
const COLUMNS: [i32; 4] = [180, 220, 170, 200];

pub struct ContainersTab {
    start: Button,
    stop: Button,
    restart: Button,
    remove: Button,
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
        let mut action = |label: &str, a: Action| {
            let mut b = Button::default().with_label(label);
            b.set_callback(move |_| s.send(Msg::ContainerAction(a)));
            b.deactivate();
            buttons.fixed(&b, 80);
            b
        };
        let start = action("Start", Action::Start);
        let stop = action("Stop", Action::Stop);
        let restart = action("Restart", Action::Restart);
        let remove = action("Remove", Action::Remove);
        Frame::default();
        let mut refresh = Button::default().with_label("Refresh");
        refresh.set_callback(move |_| s.send(Msg::Refresh));
        buttons.fixed(&refresh, 90);
        buttons.end();
        col.fixed(&buttons, 28);

        let header = super::header(&COLUMNS, &["NAME", "IMAGE", "STATUS", "PORTS", "CREATED"]);
        col.fixed(&header, 22);

        let mut list = HoldBrowser::default();
        list.set_column_widths(&COLUMNS);
        list.set_callback(move |_| s.send(Msg::ContainerSelected));
        col.end();

        ContainersTab {
            start,
            stop,
            restart,
            remove,
            list,
            rows: Vec::new(),
            selected: None,
        }
    }

    /// Replaces the list, keeping the selection (by ID) and scroll position.
    pub fn set_containers(&mut self, mut rows: Vec<ContainerSummary>) {
        sort(&mut rows);
        let now = format::now();
        super::measure_with(&self.list);
        let lines: Vec<String> = rows
            .iter()
            .map(|c| row(c, now, &|text, i| super::fit(text, COLUMNS[i])))
            .collect();
        super::fill(&mut self.list, &lines, "No containers");
        self.rows = rows;
        match self.index_of_selected() {
            Some(i) => self.list.select(i as i32 + 1),
            None => self.selected = None,
        }
        self.update_buttons();
    }

    /// Records the newly selected container after a click.
    pub fn on_select(&mut self) {
        let line = self.list.value();
        self.selected = (line > 0)
            .then(|| self.rows.get(line as usize - 1))
            .flatten()
            .map(|c| c.id.clone());
        self.update_buttons();
    }

    pub fn selected(&self) -> Option<&ContainerSummary> {
        self.rows.get(self.index_of_selected()?)
    }

    /// Enables buttons by the selected container's state (SPEC §8.3).
    fn update_buttons(&mut self) {
        let (any, running) = match self.selected() {
            Some(c) => (true, c.state == "running"),
            None => (false, false),
        };
        for (b, on) in [
            (&mut self.start, any && !running),
            (&mut self.stop, running),
            (&mut self.restart, running),
            (&mut self.remove, any),
        ] {
            if on {
                b.activate();
            } else {
                b.deactivate();
            }
        }
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
