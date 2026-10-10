//! Images tab: list, remove and prune.

use crate::docker::models::ImageSummary;
use crate::format;
use crate::msg::Msg;
use fltk::{
    app::Sender,
    browser::{Browser, HoldBrowser},
    button::Button,
    frame::Frame,
    group::Flex,
    prelude::*,
};

/// Width fractions of all columns but the last (Created).
const COLUMNS: [f32; 3] = [0.45, 0.17, 0.13];
const NONE: &str = "<none>:<none>";

/// One list row: an image under one of its tags (or untagged).
pub struct ImageRow {
    pub tag: Option<String>,
    pub id: String,
    pub size: i64,
    pub created: i64,
}

impl ImageRow {
    /// What Docker should delete: the tag (untags) or, for untagged images, the ID.
    pub fn reference(&self) -> &str {
        self.tag.as_deref().unwrap_or(&self.id)
    }

    /// Name for dialogs and the status bar.
    pub fn display_name(&self) -> String {
        match &self.tag {
            Some(t) => t.clone(),
            None => format!("{NONE} ({})", short_id(&self.id)),
        }
    }
}

pub struct ImagesTab {
    remove: Button,
    header: Browser,
    list: HoldBrowser,
    rows: Vec<ImageRow>,
    /// Selected row's [`ImageRow::reference`].
    selected: Option<String>,
}

impl ImagesTab {
    /// Builds the tab inside the current group (the `Tabs`).
    pub fn new(s: Sender<Msg>) -> Self {
        let mut col = Flex::default_fill().column().with_label("   Images   ");
        col.set_margin(8);
        col.set_pad(6);

        let mut buttons = Flex::default().row();
        let mut remove = Button::default().with_label("Remove");
        remove.set_callback(move |_| s.send(Msg::RemoveImage));
        remove.set_label_color(super::theme::RED);
        remove.deactivate();
        buttons.fixed(&remove, 80);
        Frame::default();
        let mut prune = Button::default().with_label("Prune…");
        prune.set_callback(move |_| s.send(Msg::PruneImages));
        buttons.fixed(&prune, 90);
        let mut refresh = Button::default().with_label("Refresh");
        refresh.set_callback(move |_| s.send(Msg::Refresh));
        buttons.fixed(&refresh, 90);
        buttons.end();
        col.fixed(&buttons, 30);

        let widths = super::column_widths(880, &COLUMNS);
        let header = super::header(&widths, &["REPOSITORY:TAG", "IMAGE ID", "SIZE", "CREATED"]);
        col.fixed(&header, 22);

        let mut list = HoldBrowser::default();
        super::style_list(&mut list, &widths);
        list.set_callback(move |_| s.send(Msg::ImageSelected));
        col.end();

        ImagesTab {
            remove,
            header,
            list,
            rows: Vec::new(),
            selected: None,
        }
    }

    /// Replaces the list, keeping the selection and scroll position.
    pub fn set_images(&mut self, images: Vec<ImageSummary>) {
        self.rows = rows(images);
        self.render();
    }

    /// Redraws the rows, fitting cells to the current column widths.
    pub fn render(&mut self) {
        let widths = super::column_widths(self.list.w(), &COLUMNS);
        super::set_columns(&mut self.header, &mut self.list, &widths);
        let now = format::now();
        super::measure_with(&self.list);
        let lines: Vec<String> = self
            .rows
            .iter()
            .map(|r| {
                format!(
                    "@.{}\t@.{}\t@.{}\t@.{}",
                    super::fit(r.tag.as_deref().unwrap_or(NONE), widths[0]),
                    short_id(&r.id),
                    format::size(r.size),
                    format::relative_time(r.created, now),
                )
            })
            .collect();
        super::fill(&mut self.list, &lines, "No images");
        match self.index_of_selected() {
            Some(i) => self.list.select(i as i32 + 1),
            None => self.selected = None,
        }
        self.update_buttons();
    }

    /// Records the newly selected row after a click.
    pub fn on_select(&mut self) {
        let line = self.list.value();
        self.selected = (line > 0)
            .then(|| self.rows.get(line as usize - 1))
            .flatten()
            .map(|r| r.reference().to_string());
        self.update_buttons();
    }

    pub fn selected(&self) -> Option<&ImageRow> {
        self.rows.get(self.index_of_selected()?)
    }

    fn update_buttons(&mut self) {
        if self.selected.is_some() {
            self.remove.activate();
        } else {
            self.remove.deactivate();
        }
    }

    fn index_of_selected(&self) -> Option<usize> {
        let key = self.selected.as_deref()?;
        self.rows.iter().position(|r| r.reference() == key)
    }
}

/// First 12 hex characters after `sha256:`.
fn short_id(id: &str) -> &str {
    let hex = id.strip_prefix("sha256:").unwrap_or(id);
    &hex[..hex.len().min(12)]
}

/// One row per tag, sorted by repository:tag with untagged rows last.
fn rows(images: Vec<ImageSummary>) -> Vec<ImageRow> {
    let mut rows = Vec::new();
    for img in images {
        let tags: Vec<String> = img.repo_tags.into_iter().filter(|t| t != NONE).collect();
        let tags = if tags.is_empty() {
            vec![None]
        } else {
            tags.into_iter().map(Some).collect()
        };
        for tag in tags {
            rows.push(ImageRow {
                tag,
                id: img.id.clone(),
                size: img.size,
                created: img.created,
            });
        }
    }
    rows.sort_by_cached_key(|r| {
        (
            r.tag.is_none(),
            r.tag.as_ref().map(|t| t.to_lowercase()),
            r.id.clone(),
        )
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(id: &str, tags: &[&str]) -> ImageSummary {
        ImageSummary {
            id: id.to_string(),
            repo_tags: tags.iter().map(|t| t.to_string()).collect(),
            size: 1,
            created: 1,
        }
    }

    #[test]
    fn one_row_per_tag_none_last() {
        let r = rows(vec![
            img("sha256:aaa", &[NONE]),
            img("sha256:bbb", &["redis:7", "Alpine:3"]),
            img("sha256:ccc", &[]),
            img("sha256:ddd", &["nginx:alpine"]),
        ]);
        let names: Vec<String> = r.iter().map(|r| r.reference().to_string()).collect();
        assert_eq!(
            names,
            [
                "Alpine:3",
                "nginx:alpine",
                "redis:7",
                "sha256:aaa",
                "sha256:ccc"
            ]
        );
    }

    #[test]
    fn short_ids_and_names() {
        assert_eq!(short_id("sha256:0123456789abcdef"), "0123456789ab");
        assert_eq!(short_id("abc"), "abc");
        let untagged = ImageRow {
            tag: None,
            id: "sha256:0123456789abcdef".into(),
            size: 0,
            created: 0,
        };
        assert_eq!(untagged.display_name(), "<none>:<none> (0123456789ab)");
        assert_eq!(untagged.reference(), "sha256:0123456789abcdef");
    }
}
