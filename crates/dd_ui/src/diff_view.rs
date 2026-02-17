use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    canvas, px, App, Bounds, Context, HighlightStyle, Hsla, Pixels, SharedString, StyledText,
    Window,
};
use gpui_component::{scroll::ScrollableElement, v_flex, ActiveTheme};

use dd_git::{
    split_hunk_lines, CommitInfo, DiffLine, FileDiff, Hunk, LineOrigin, SignatureStatus, SplitRow,
};

use crate::syntax;
use crate::theme::DiffTheme;

const SPLIT_VIEW_MIN_WIDTH: f32 = 1000.0;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffViewMode {
    Unified,
    Split,
}

#[derive(Debug, Clone, Copy)]
enum SplitSide {
    Left,
    Right,
}

pub struct DiffView {
    diffs: Vec<FileDiff>,
    commit_info: Option<CommitInfo>,
    signature_status: Option<SignatureStatus>,
    error_message: Option<String>,
    mode: DiffViewMode,
}

impl DiffView {
    pub fn new_empty() -> Self {
        Self {
            diffs: Vec::new(),
            commit_info: None,
            signature_status: None,
            error_message: None,
            mode: DiffViewMode::Unified,
        }
    }

    pub fn diffs(&self) -> &[FileDiff] {
        &self.diffs
    }

    pub fn commit_info(&self) -> Option<&CommitInfo> {
        self.commit_info.as_ref()
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn set_diffs(&mut self, diffs: Vec<FileDiff>, cx: &mut Context<Self>) {
        self.diffs = diffs;
        self.commit_info = None;
        self.signature_status = None;
        self.error_message = None;
        cx.notify();
    }

    pub fn set_commit_data(
        &mut self,
        commit: CommitInfo,
        signature: SignatureStatus,
        diffs: Vec<FileDiff>,
        cx: &mut Context<Self>,
    ) {
        self.commit_info = Some(commit);
        self.signature_status = Some(signature);
        self.diffs = diffs;
        self.error_message = None;
        cx.notify();
    }

    pub fn set_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.error_message = Some(message);
        self.diffs.clear();
        self.commit_info = None;
        self.signature_status = None;
        cx.notify();
    }

    // -- Shared helpers ---------------------------------------------------

    fn render_file_header(&self, file: &FileDiff, cx: &Context<Self>) -> gpui::Div {
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

        gpui::div()
            .px_3()
            .py_1()
            .bg(cx.theme().muted)
            .text_sm()
            .font_weight(gpui::FontWeight::BOLD)
            .child(path_display)
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

    // -- Unified rendering ------------------------------------------------

    fn render_unified(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let file_elements: Vec<_> = self
            .diffs
            .iter()
            .map(|file| self.render_file_diff(file, cx))
            .collect();

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scrollbar()
            .gap_2()
            .children(file_elements)
            .into_any_element()
    }

    fn render_file_diff(&self, file: &FileDiff, cx: &Context<Self>) -> impl IntoElement {
        let hunk_elements: Vec<_> = file
            .hunks
            .iter()
            .map(|hunk| self.render_hunk(hunk, &file.path, cx))
            .collect();

        v_flex()
            .w_full()
            .gap_1()
            .child(self.render_file_header(file, cx))
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

    // -- Commit header -----------------------------------------------------
}

fn compute_stats(diffs: &[FileDiff]) -> (usize, usize, usize) {
    let files = diffs.len();
    let mut additions = 0usize;
    let mut deletions = 0usize;
    for file in diffs {
        for hunk in &file.hunks {
            for line in &hunk.lines {
                match line.origin {
                    LineOrigin::Addition => additions += 1,
                    LineOrigin::Deletion => deletions += 1,
                    LineOrigin::Context => {}
                }
            }
        }
    }
    (files, additions, deletions)
}

fn format_commit_date(timestamp: i64) -> String {
    use chrono::{DateTime, Local, TimeZone};
    match Local.timestamp_opt(timestamp, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%a, %b %-d, %Y, %-I:%M %p").to_string(),
        _ => match DateTime::from_timestamp(timestamp, 0) {
            Some(dt) => dt.format("%a, %b %-d, %Y, %-I:%M %p UTC").to_string(),
            None => "unknown".to_string(),
        },
    }
}

const LABEL_WIDTH: f32 = 100.0;

impl DiffView {
    fn render_commit_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let commit = self.commit_info.as_ref().unwrap();
        let signature = self.signature_status.unwrap_or(SignatureStatus::None);

        let parents_str = if commit.parent_oids.is_empty() {
            "(root commit)".to_string()
        } else {
            commit
                .parent_oids
                .iter()
                .map(|p| &p[..7.min(p.len())])
                .collect::<Vec<_>>()
                .join(", ")
        };

        let (files, additions, deletions) = compute_stats(&self.diffs);
        let stats_str = format!(
            "{} file{}, +{} addition{}, -{} deletion{}",
            files,
            if files == 1 { "" } else { "s" },
            additions,
            if additions == 1 { "" } else { "s" },
            deletions,
            if deletions == 1 { "" } else { "s" },
        );

        let sig_color = match signature {
            SignatureStatus::Good => theme.success,
            SignatureStatus::Bad => theme.danger,
            _ => theme.muted_foreground,
        };

        let mut header = v_flex().w_full().px_3().py_2().gap_0p5();

        let rows: Vec<(&str, String, Option<Hsla>)> = vec![
            ("Commit", commit.oid.clone(), None),
            ("Tree", commit.tree_oid.clone(), None),
            (
                "Author",
                format!("{} <{}>", commit.author_name, commit.author_email),
                None,
            ),
            (
                "Committer",
                format!("{} <{}>", commit.committer_name, commit.committer_email),
                None,
            ),
            ("Date", format_commit_date(commit.date), None),
            ("Parents", parents_str, None),
            ("Signature", signature.label().to_string(), Some(sig_color)),
            ("Stats", stats_str, None),
        ];

        for (label, value, color) in rows {
            header = header.child(
                gpui::div()
                    .flex()
                    .w_full()
                    .text_xs()
                    .font_family(theme.font_family.clone())
                    .child(
                        gpui::div()
                            .w(gpui::px(LABEL_WIDTH))
                            .flex_shrink_0()
                            .text_right()
                            .pr_2()
                            .text_color(theme.muted_foreground)
                            .child(format!("{}:", label)),
                    )
                    .child(
                        gpui::div()
                            .text_color(color.unwrap_or(theme.foreground))
                            .child(value),
                    ),
            );
        }

        header = header.child(
            v_flex()
                .mt_2()
                .px_1()
                .gap_0p5()
                .child(
                    gpui::div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.foreground)
                        .child(commit.subject.clone()),
                )
                .when(!commit.body.is_empty(), |el| {
                    el.child(
                        gpui::div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(commit.body.clone()),
                    )
                }),
        );

        header = header.child(
            gpui::div()
                .mt_2()
                .w_full()
                .h(gpui::px(1.0))
                .bg(theme.border),
        );

        header
    }

    // -- Split rendering --------------------------------------------------

    fn render_split(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let file_elements: Vec<_> = self
            .diffs
            .iter()
            .map(|file| self.render_file_diff_split(file, cx))
            .collect();

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scrollbar()
            .gap_2()
            .children(file_elements)
            .into_any_element()
    }

    fn render_file_diff_split(&self, file: &FileDiff, cx: &Context<Self>) -> impl IntoElement {
        let hunk_elements: Vec<_> = file
            .hunks
            .iter()
            .map(|hunk| self.render_hunk_split(hunk, &file.path, cx))
            .collect();

        v_flex()
            .w_full()
            .gap_1()
            .child(self.render_file_header(file, cx))
            .children(hunk_elements)
    }

    fn render_hunk_split(
        &self,
        hunk: &Hunk,
        file_path: &str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let diff_theme = DiffTheme::from_cx(cx);
        let theme = cx.theme();
        let rows = split_hunk_lines(&hunk.lines);

        let row_elements: Vec<_> = rows
            .iter()
            .map(|row| self.render_split_row(row, file_path, &diff_theme, cx))
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
            .children(row_elements)
    }

    fn render_split_row(
        &self,
        row: &SplitRow,
        file_path: &str,
        diff_theme: &DiffTheme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        gpui::div()
            .w_full()
            .flex()
            .text_xs()
            .line_height(gpui::rems(1.0))
            .font_family(theme.font_family.clone())
            .child(self.render_split_half(
                row.left.as_deref(),
                SplitSide::Left,
                file_path,
                diff_theme,
                cx,
            ))
            .child(gpui::div().w(px(1.0)).flex_shrink_0().bg(theme.border))
            .child(self.render_split_half(
                row.right.as_deref(),
                SplitSide::Right,
                file_path,
                diff_theme,
                cx,
            ))
    }

    fn render_split_half(
        &self,
        line: Option<&DiffLine>,
        side: SplitSide,
        file_path: &str,
        diff_theme: &DiffTheme,
        cx: &Context<Self>,
    ) -> gpui::Div {
        let theme = cx.theme();

        let Some(line) = line else {
            return gpui::div()
                .flex_1()
                .flex()
                .overflow_x_hidden()
                .bg(theme.background);
        };

        let bg_color = match line.origin {
            LineOrigin::Addition => diff_theme.add_bg,
            LineOrigin::Deletion => diff_theme.del_bg,
            LineOrigin::Context => diff_theme.ctx_bg,
        };

        let line_no_str = match side {
            SplitSide::Left => line
                .old_line_no
                .map(|n| format!("{:>4}", n))
                .unwrap_or_else(|| "    ".to_string()),
            SplitSide::Right => line
                .new_line_no
                .map(|n| format!("{:>4}", n))
                .unwrap_or_else(|| "    ".to_string()),
        };

        gpui::div()
            .flex_1()
            .flex()
            .overflow_x_hidden()
            .bg(bg_color)
            .child(
                gpui::div()
                    .w(px(48.0))
                    .flex_shrink_0()
                    .text_color(diff_theme.line_number_fg)
                    .text_right()
                    .px_1()
                    .child(line_no_str),
            )
            .child(
                gpui::div()
                    .px_1()
                    .overflow_x_hidden()
                    .whitespace_nowrap()
                    .child(self.render_content(line, file_path, diff_theme, cx)),
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

        let weak = cx.entity().downgrade();

        let content = match self.mode {
            DiffViewMode::Unified => self.render_unified(cx),
            DiffViewMode::Split => self.render_split(cx),
        };

        // Measure available width during layout and update mode for the next
        // frame. The content uses the previous frame's mode (defaults to Unified);
        // this is a standard one-frame-behind pattern in GPUI that converges
        // after a single extra render cycle. Oscillation cannot occur because
        // the diff view's width is determined by its parent resizable panel,
        // not by the diff content.
        v_flex()
            .size_full()
            .child(
                canvas(
                    move |bounds: Bounds<Pixels>, _window: &mut Window, app: &mut App| {
                        let new_mode = if bounds.size.width >= px(SPLIT_VIEW_MIN_WIDTH) {
                            DiffViewMode::Split
                        } else {
                            DiffViewMode::Unified
                        };
                        let _ = weak.update(app, |view: &mut DiffView, cx| {
                            if view.mode != new_mode {
                                view.mode = new_mode;
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .w_full()
                .h(px(0.)),
            )
            .when(self.commit_info.is_some(), |el| {
                el.child(self.render_commit_header(cx))
            })
            .child(content)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dd_git::{FileStatus, Hunk, SignatureStatus};

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

    fn mock_commit() -> CommitInfo {
        CommitInfo {
            oid: "abc123def456".into(),
            short_oid: "abc123d".into(),
            tree_oid: "tree111aaa".into(),
            author_name: "Alice".into(),
            author_email: "alice@example.com".into(),
            date: 1700000000,
            committer_name: "Alice".into(),
            committer_email: "alice@example.com".into(),
            committer_date: 1700000000,
            subject: "feat: add login".into(),
            body: "Detailed description of the change.".into(),
            parent_oids: vec!["def456abc789".into()],
        }
    }

    #[test]
    fn test_compute_stats() {
        let diffs = mock_diffs();
        let (files, additions, deletions) = compute_stats(&diffs);
        assert_eq!(files, 1);
        assert_eq!(additions, 2);
        assert_eq!(deletions, 1);
    }

    #[test]
    fn test_compute_stats_empty() {
        let (files, additions, deletions) = compute_stats(&[]);
        assert_eq!(files, 0);
        assert_eq!(additions, 0);
        assert_eq!(deletions, 0);
    }

    #[test]
    fn test_format_commit_date() {
        let formatted = format_commit_date(1700000000);
        // Should produce a human-readable date string
        assert!(!formatted.is_empty());
        assert_ne!(formatted, "unknown");
    }

    #[test]
    fn test_format_commit_date_invalid() {
        let formatted = format_commit_date(i64::MIN);
        assert_eq!(formatted, "unknown");
    }

    #[test]
    fn test_signature_status_from_git_char() {
        assert_eq!(SignatureStatus::from_git_char('G'), SignatureStatus::Good);
        assert_eq!(SignatureStatus::from_git_char('B'), SignatureStatus::Bad);
        assert_eq!(
            SignatureStatus::from_git_char('U'),
            SignatureStatus::Unknown
        );
        assert_eq!(SignatureStatus::from_git_char('N'), SignatureStatus::None);
        assert_eq!(SignatureStatus::from_git_char('?'), SignatureStatus::None);
    }

    #[test]
    fn test_signature_status_label() {
        assert_eq!(SignatureStatus::Good.label(), "Valid");
        assert_eq!(SignatureStatus::Bad.label(), "Invalid");
        assert_eq!(SignatureStatus::Unknown.label(), "Unknown");
        assert_eq!(SignatureStatus::None.label(), "None");
    }

    #[gpui::test]
    fn test_set_commit_data(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| DiffView::new_empty());

        window
            .update(cx, |view, _window, cx| {
                view.set_commit_data(mock_commit(), SignatureStatus::None, mock_diffs(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(view.commit_info().is_some());
                assert_eq!(view.commit_info().unwrap().subject, "feat: add login");
                assert_eq!(view.diffs().len(), 1);
                assert!(view.error_message().is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_set_error_clears_commit_info(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| DiffView::new_empty());

        window
            .update(cx, |view, _window, cx| {
                view.set_commit_data(mock_commit(), SignatureStatus::Good, mock_diffs(), cx);
            })
            .unwrap();

        window
            .update(cx, |view, _window, cx| {
                view.set_error("oops".into(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(view.commit_info().is_none());
                assert!(view.diffs().is_empty());
                assert_eq!(view.error_message(), Some("oops"));
            })
            .unwrap();
    }
}
