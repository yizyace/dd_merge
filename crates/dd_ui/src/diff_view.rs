use std::ops::Range;

use gpui::prelude::*;
use gpui::{Context, HighlightStyle, Hsla, SharedString, StyledText, Window};
use gpui_component::{scroll::ScrollableElement, v_flex, ActiveTheme};

use dd_git::{DiffLine, FileDiff, Hunk, LineOrigin};

use crate::syntax;
use crate::theme::DiffTheme;

fn fallback_color(
    origin: &LineOrigin,
    diff_theme: &DiffTheme,
    theme: &gpui_component::Theme,
) -> Hsla {
    match origin {
        LineOrigin::Context => diff_theme.ctx_fg,
        _ => theme.foreground,
    }
}

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

        let path_display = if let Some(ref old) = file.old_path {
            format!("{} {} \u{2192} {}", status_label, old, file.path)
        } else {
            format!("{} {}", status_label, file.path)
        };

        let hunk_elements: Vec<_> = file
            .hunks
            .iter()
            .map(|hunk| self.render_hunk(hunk, &file.path, cx))
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
                    .child(path_display),
            )
            .children(hunk_elements)
    }

    fn render_hunk(&self, hunk: &Hunk, file_path: &str, cx: &Context<Self>) -> impl IntoElement {
        let diff_theme = DiffTheme::from_cx(cx);
        let theme = cx.theme();

        let line_elements: Vec<_> = hunk
            .lines
            .iter()
            .map(|line| self.render_diff_line(line, file_path, &diff_theme, cx))
            .collect();

        v_flex()
            .w_full()
            .child(
                gpui::div()
                    .px_3()
                    .py_0p5()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .bg(theme.muted)
                    .child(hunk.header.clone()),
            )
            .children(line_elements)
    }

    fn render_diff_line(
        &self,
        line: &DiffLine,
        file_path: &str,
        diff_theme: &DiffTheme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        let (prefix, bg_color) = match line.origin {
            LineOrigin::Addition => ("+", diff_theme.add_bg),
            LineOrigin::Deletion => ("-", diff_theme.del_bg),
            LineOrigin::Context => (" ", diff_theme.ctx_bg),
        };

        let fg = fallback_color(&line.origin, diff_theme, theme);

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
            .overflow_x_hidden()
            .bg(bg_color)
            .text_xs()
            .line_height(gpui::rems(1.0))
            .font_family(theme.font_family.clone())
            .child(
                gpui::div()
                    .w(gpui::px(48.0))
                    .flex_shrink_0()
                    .text_color(diff_theme.line_number_fg)
                    .text_right()
                    .px_1()
                    .child(old_str),
            )
            .child(
                gpui::div()
                    .w(gpui::px(48.0))
                    .flex_shrink_0()
                    .text_color(diff_theme.line_number_fg)
                    .text_right()
                    .px_1()
                    .child(new_str),
            )
            .child(
                gpui::div()
                    .flex_shrink_0()
                    .text_color(fg)
                    .child(prefix.to_string()),
            )
            .child(
                gpui::div()
                    .px_1()
                    .overflow_x_hidden()
                    .child(self.render_content(line, file_path, diff_theme, cx)),
            )
    }

    fn render_content(
        &self,
        line: &DiffLine,
        file_path: &str,
        diff_theme: &DiffTheme,
        cx: &Context<Self>,
    ) -> StyledText {
        let theme = cx.theme();
        let content = &line.content;

        let fg = fallback_color(&line.origin, diff_theme, theme);
        let is_dark = theme.background.l < 0.5;

        let highlight_bg = match line.origin {
            LineOrigin::Addition => diff_theme.add_highlight_bg,
            LineOrigin::Deletion => diff_theme.del_highlight_bg,
            LineOrigin::Context => diff_theme.ctx_bg,
        };

        let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();

        // Syntax foreground colors
        let syntax_highlights = syntax::highlight_line(file_path, content, fg, is_dark);
        for sh in &syntax_highlights {
            highlights.push((
                sh.range.clone(),
                HighlightStyle {
                    color: Some(sh.color),
                    ..Default::default()
                },
            ));
        }

        // Change-span background colors
        for cs in &line.change_spans {
            highlights.push((
                cs.start..cs.end,
                HighlightStyle {
                    background_color: Some(highlight_bg),
                    ..Default::default()
                },
            ));
        }

        StyledText::new(SharedString::from(content.clone())).with_highlights(highlights)
    }
}

impl DiffView {
    fn render_unified(&self, cx: &Context<Self>) -> gpui::AnyElement {
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

        self.render_unified(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dd_git::{FileStatus, Hunk};

    fn mock_diffs() -> Vec<FileDiff> {
        vec![FileDiff {
            path: "src/main.rs".into(),
            old_path: None,
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
