use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    uniform_list, Context, MouseButton, MouseDownEvent, ScrollStrategy, UniformListScrollHandle,
    Window,
};
use gpui_component::{v_flex, ActiveTheme};

use dd_git::CommitInfo;

const LOAD_MORE_THRESHOLD: usize = 20;

pub struct CommitList {
    commits: Vec<CommitInfo>,
    selected_index: Option<usize>,
    scroll_handle: UniformListScrollHandle,
    loading_more: bool,
    all_loaded: bool,
    batch_size: usize,
    #[allow(clippy::type_complexity)]
    on_select: Option<Box<dyn Fn(&CommitInfo, &mut Window, &mut Context<Self>) + 'static>>,
    #[allow(clippy::type_complexity)]
    on_load_more: Option<Box<dyn Fn(&str) -> Vec<CommitInfo> + 'static>>,
}

impl CommitList {
    pub fn new_empty() -> Self {
        Self {
            commits: Vec::new(),
            selected_index: None,
            scroll_handle: UniformListScrollHandle::new(),
            loading_more: false,
            all_loaded: false,
            batch_size: 0,
            on_select: None,
            on_load_more: None,
        }
    }

    pub fn set_commits(&mut self, commits: Vec<CommitInfo>, cx: &mut Context<Self>) {
        self.commits = commits;
        self.selected_index = None;
        self.loading_more = false;
        self.all_loaded = false;
        cx.notify();
    }

    pub fn commits(&self) -> &[CommitInfo] {
        &self.commits
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn on_select(
        &mut self,
        callback: impl Fn(&CommitInfo, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.on_select = Some(Box::new(callback));
    }

    pub fn on_load_more(
        &mut self,
        batch_size: usize,
        callback: impl Fn(&str) -> Vec<CommitInfo> + 'static,
    ) {
        self.batch_size = batch_size;
        self.on_load_more = Some(Box::new(callback));
    }

    pub fn mark_all_loaded(&mut self) {
        self.all_loaded = true;
    }

    pub fn select_commit(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_index == Some(index) {
            return;
        }
        if let Some(commit) = self.commits.get(index) {
            self.selected_index = Some(index);
            self.scroll_handle
                .scroll_to_item(index, ScrollStrategy::Center);
            if let Some(ref on_select) = self.on_select {
                on_select(commit, window, cx);
            }
        }
        cx.notify();
    }

    fn check_load_more(&mut self, visible_end: usize, cx: &mut Context<Self>) {
        if self.loading_more || self.all_loaded || self.commits.is_empty() {
            return;
        }
        let remaining = self.commits.len().saturating_sub(visible_end);
        if remaining > LOAD_MORE_THRESHOLD {
            return;
        }

        let last_oid = match self.commits.last() {
            Some(c) => c.oid.clone(),
            None => return,
        };
        let new_commits = match self.on_load_more {
            Some(ref loader) => loader(&last_oid),
            None => return,
        };

        self.loading_more = true;
        if new_commits.len() < self.batch_size {
            self.all_loaded = true;
        }
        self.commits.extend(new_commits);
        self.loading_more = false;
        cx.notify();
    }

    fn format_date(timestamp: i64) -> String {
        use chrono::{DateTime, Utc};
        let dt = DateTime::<Utc>::from_timestamp(timestamp, 0);
        match dt {
            Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
            None => "unknown".to_string(),
        }
    }

    fn render_commit_row(
        &self,
        index: usize,
        commit: &CommitInfo,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self.selected_index == Some(index);
        let subject = commit.subject.clone();
        let author = commit.author_name.clone();
        let date = Self::format_date(commit.date);
        let short_oid = commit.short_oid.clone();

        gpui::div()
            .id(gpui::ElementId::Integer(index as u64))
            .w_full()
            .px_3()
            .py_1()
            .cursor_pointer()
            .when(is_selected, |el| el.bg(cx.theme().accent))
            .hover(|el| {
                if is_selected {
                    el
                } else {
                    el.bg(cx.theme().muted)
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _event: &MouseDownEvent, window, cx| {
                    view.select_commit(index, window, cx);
                }),
            )
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(if is_selected {
                                cx.theme().accent_foreground
                            } else {
                                cx.theme().foreground
                            })
                            .child(subject),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .gap_2()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(short_oid)
                            .child(author)
                            .child(date),
                    ),
            )
    }
}

impl Render for CommitList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scroll_handle = self.scroll_handle.clone();
        let item_count = self.commits.len();

        uniform_list(
            "commit-list",
            item_count,
            cx.processor(|this, range: Range<usize>, window, cx| {
                let visible_end = range.end;
                let should_check =
                    !this.loading_more && !this.all_loaded && !this.commits.is_empty();

                let items: Vec<_> = range
                    .map(|ix| {
                        let commit = &this.commits[ix];
                        this.render_commit_row(ix, commit, cx)
                    })
                    .collect();

                if should_check {
                    cx.defer_in(window, move |this, _window, cx| {
                        this.check_load_more(visible_end, cx);
                    });
                }

                items
            }),
        )
        .h_full()
        .w_full()
        .track_scroll(scroll_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_commits() -> Vec<CommitInfo> {
        vec![
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
                body: String::new(),
                parent_oids: vec![],
            },
            CommitInfo {
                oid: "def456abc789".into(),
                short_oid: "def456a".into(),
                tree_oid: "tree222bbb".into(),
                author_name: "Bob".into(),
                author_email: "bob@example.com".into(),
                date: 1699999000,
                committer_name: "Bob".into(),
                committer_email: "bob@example.com".into(),
                committer_date: 1699999000,
                subject: "fix: typo".into(),
                body: String::new(),
                parent_oids: vec!["abc123def456".into()],
            },
        ]
    }

    #[test]
    fn test_commit_list_data() {
        let commits = mock_commits();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "feat: add login");
        assert_eq!(commits[1].author_name, "Bob");
    }

    #[test]
    fn test_format_date() {
        let formatted = CommitList::format_date(1700000000);
        assert!(formatted.starts_with("2023-11-14"));
    }

    #[test]
    fn test_format_date_invalid_timestamp() {
        let formatted = CommitList::format_date(i64::MIN);
        assert_eq!(formatted, "unknown");
    }

    #[gpui::test]
    fn test_set_commits_and_select_triggers_callback(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));

        let selected_oid = std::rc::Rc::new(std::cell::Cell::new(String::new()));
        let selected_oid_clone = selected_oid.clone();

        let window = cx.add_window(|_window, _cx| CommitList::new_empty());

        window
            .update(cx, |list, _window, cx| {
                list.set_commits(mock_commits(), cx);
                list.on_select(move |commit, _window, _cx| {
                    selected_oid_clone.set(commit.oid.clone());
                });
            })
            .unwrap();

        window
            .update(cx, |list, window, cx| {
                list.select_commit(0, window, cx);
            })
            .unwrap();

        assert_eq!(selected_oid.take(), "abc123def456");

        window
            .read_with(cx, |list, _cx| {
                assert_eq!(list.selected_index(), Some(0));
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_select_commit_out_of_bounds_leaves_none(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));

        let window = cx.add_window(|_window, _cx| CommitList::new_empty());

        window
            .update(cx, |list, _window, cx| {
                list.set_commits(mock_commits(), cx);
            })
            .unwrap();

        window
            .update(cx, |list, window, cx| {
                list.select_commit(99, window, cx);
            })
            .unwrap();

        window
            .read_with(cx, |list, _cx| {
                assert_eq!(list.selected_index(), None);
            })
            .unwrap();
    }
}
