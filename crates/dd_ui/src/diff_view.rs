use gpui::prelude::*;
use gpui::{Context, Window};
use gpui_component::{scroll::ScrollableElement, v_flex, ActiveTheme};

use dd_git::{DiffLine, FileDiff, Hunk, LineOrigin};

pub struct DiffView {
    diffs: Vec<FileDiff>,
    error_message: Option<String>,
}

impl DiffView {
    pub fn new_empty() -> Self {
        Self {
            diffs: Vec::new(),
            error_message: None,
        }
    }

    pub fn diffs(&self) -> &[FileDiff] {
        &self.diffs
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn set_diffs(&mut self, diffs: Vec<FileDiff>, cx: &mut Context<Self>) {
        self.diffs = diffs;
        self.error_message = None;
        cx.notify();
    }

    pub fn set_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.error_message = Some(message);
        self.diffs.clear();
        cx.notify();
    }

    fn render_file_diff(&self, file: &FileDiff, cx: &Context<Self>) -> impl IntoElement {
        let status_label = match file.status {
            dd_git::FileStatus::Added => "A",
            dd_git::FileStatus::Deleted => "D",
            dd_git::FileStatus::Modified => "M",
            dd_git::FileStatus::Renamed => "R",
        };

        let hunk_elements: Vec<_> = file
            .hunks
            .iter()
            .map(|hunk| self.render_hunk(hunk, cx))
            .collect();

        v_flex()
            .w_full()
            .gap_1()
            .child(
                gpui::div()
                    .px_3()
                    .py_1()
                    .bg(cx.theme().muted)
                    .text_sm()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(format!("{} {}", status_label, file.path)),
            )
            .children(hunk_elements)
    }

    fn render_hunk(&self, hunk: &Hunk, cx: &Context<Self>) -> impl IntoElement {
        let line_elements: Vec<_> = hunk
            .lines
            .iter()
            .map(|line| self.render_diff_line(line, cx))
            .collect();

        v_flex()
            .w_full()
            .child(
                gpui::div()
                    .px_3()
                    .py_0p5()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .bg(cx.theme().muted)
                    .child(hunk.header.clone()),
            )
            .children(line_elements)
    }

    fn render_diff_line(&self, line: &DiffLine, cx: &Context<Self>) -> impl IntoElement {
        let (prefix, bg_color, text_color) = match line.origin {
            LineOrigin::Addition => (
                "+",
                gpui::hsla(120.0 / 360.0, 0.4, 0.15, 1.0),
                gpui::hsla(120.0 / 360.0, 0.6, 0.7, 1.0),
            ),
            LineOrigin::Deletion => (
                "-",
                gpui::hsla(0.0, 0.4, 0.15, 1.0),
                gpui::hsla(0.0, 0.6, 0.7, 1.0),
            ),
            LineOrigin::Context => (
                " ",
                gpui::hsla(0.0, 0.0, 0.0, 0.0),
                cx.theme().muted_foreground,
            ),
        };

        let old_str = line
            .old_line_no
            .map(|n| format!("{:>4}", n))
            .unwrap_or_else(|| "    ".to_string());
        let new_str = line
            .new_line_no
            .map(|n| format!("{:>4}", n))
            .unwrap_or_else(|| "    ".to_string());

        gpui::div()
            .w_full()
            .flex()
            .bg(bg_color)
            .text_xs()
            .font_family("monospace")
            .child(
                gpui::div()
                    .w(gpui::px(40.0))
                    .text_color(cx.theme().muted_foreground)
                    .text_right()
                    .px_1()
                    .child(old_str),
            )
            .child(
                gpui::div()
                    .w(gpui::px(40.0))
                    .text_color(cx.theme().muted_foreground)
                    .text_right()
                    .px_1()
                    .child(new_str),
            )
            .child(
                gpui::div()
                    .px_1()
                    .text_color(text_color)
                    .child(format!("{}{}", prefix, line.content)),
            )
    }
}

impl Render for DiffView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(ref error) = self.error_message {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    gpui::div()
                        .text_sm()
                        .text_color(gpui::red())
                        .child(error.clone()),
                )
                .into_any_element();
        }

        if self.diffs.is_empty() {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    gpui::div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Select a commit to view its diff"),
                )
                .into_any_element();
        }

        let file_elements: Vec<_> = self
            .diffs
            .iter()
            .map(|file| self.render_file_diff(file, cx))
            .collect();

        v_flex()
            .size_full()
            .overflow_y_scrollbar()
            .gap_2()
            .children(file_elements)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dd_git::{FileStatus, Hunk};

    fn mock_diffs() -> Vec<FileDiff> {
        vec![FileDiff {
            path: "src/main.rs".into(),
            status: FileStatus::Modified,
            hunks: vec![Hunk {
                header: "@@ -1,3 +1,4 @@".into(),
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 4,
                lines: vec![
                    DiffLine {
                        origin: LineOrigin::Context,
                        content: "fn main() {".into(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                        change_spans: vec![],
                    },
                    DiffLine {
                        origin: LineOrigin::Deletion,
                        content: "    println!(\"hello\");".into(),
                        old_line_no: Some(2),
                        new_line_no: None,
                        change_spans: vec![],
                    },
                    DiffLine {
                        origin: LineOrigin::Addition,
                        content: "    println!(\"hello world\");".into(),
                        old_line_no: None,
                        new_line_no: Some(2),
                        change_spans: vec![],
                    },
                    DiffLine {
                        origin: LineOrigin::Addition,
                        content: "    println!(\"goodbye\");".into(),
                        old_line_no: None,
                        new_line_no: Some(3),
                        change_spans: vec![],
                    },
                    DiffLine {
                        origin: LineOrigin::Context,
                        content: "}".into(),
                        old_line_no: Some(3),
                        new_line_no: Some(4),
                        change_spans: vec![],
                    },
                ],
            }],
        }]
    }

    #[test]
    fn test_diff_data_model() {
        let diffs = mock_diffs();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "src/main.rs");
        assert_eq!(diffs[0].hunks.len(), 1);
        assert_eq!(diffs[0].hunks[0].lines.len(), 5);
    }

    #[gpui::test]
    fn test_set_error_clears_diffs(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| DiffView::new_empty());

        window
            .update(cx, |view, _window, cx| {
                view.set_diffs(mock_diffs(), cx);
            })
            .unwrap();

        window
            .update(cx, |view, _window, cx| {
                view.set_error("something broke".into(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.error_message(), Some("something broke"));
                assert!(view.diffs().is_empty());
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_set_diffs_clears_error(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| DiffView::new_empty());

        window
            .update(cx, |view, _window, cx| {
                view.set_error("an error".into(), cx);
            })
            .unwrap();

        window
            .update(cx, |view, _window, cx| {
                view.set_diffs(mock_diffs(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(view.error_message().is_none());
                assert_eq!(view.diffs().len(), 1);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_set_diffs_populates_data(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));

        let window = cx.add_window(|_window, _cx| DiffView::new_empty());

        // Initially empty
        window
            .read_with(cx, |view, _cx| {
                assert!(view.diffs().is_empty());
            })
            .unwrap();

        // Set diffs
        window
            .update(cx, |view, _window, cx| {
                view.set_diffs(mock_diffs(), cx);
            })
            .unwrap();

        // Verify populated
        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.diffs().len(), 1);
                assert_eq!(view.diffs()[0].path, "src/main.rs");
                assert_eq!(view.diffs()[0].hunks[0].lines.len(), 5);
            })
            .unwrap();
    }
}
