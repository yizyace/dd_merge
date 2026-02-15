use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{actions, Context, Entity, PathPromptOptions, Window};
use gpui_component::{button::Button, v_flex, ActiveTheme};

use dd_core::{AppState, Session};

use crate::repo_view::RepoView;
use crate::tab_bar::{TabBar, TabInfo};

actions!(
    dd_merge,
    [OpenRepository, Quit, CloseTab, NextTab, PreviousTab]
);

pub struct AppView {
    state: AppState,
    repo_views: Vec<Entity<RepoView>>,
    tab_bar: Entity<TabBar>,
    error_message: Option<String>,
}

impl AppView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = Session::load().ok().flatten().unwrap_or_default();

        // Filter out repos that no longer exist or aren't valid git repos
        state
            .repos
            .retain(|tab| dd_git::Repository::open(&tab.path).is_ok());
        state.active_tab = state.active_tab.min(state.repos.len().saturating_sub(1));

        let repo_views: Vec<_> = state
            .repos
            .iter()
            .map(|tab| {
                let path = tab.path.clone();
                cx.new(|cx| RepoView::new(path, cx))
            })
            .collect();

        let tab_bar = cx.new(|_cx| TabBar::new());

        let mut view = Self {
            state,
            repo_views,
            tab_bar,
            error_message: None,
        };
        view.setup_tab_bar(cx);
        view.sync_tab_bar(cx);
        view
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn repo_view_count(&self) -> usize {
        self.repo_views.len()
    }

    pub fn tab_bar(&self) -> &Entity<TabBar> {
        &self.tab_bar
    }

    fn setup_tab_bar(&mut self, cx: &mut Context<Self>) {
        let this = cx.entity().downgrade();

        self.tab_bar.update(cx, |bar: &mut TabBar, _cx| {
            let this_select = this.clone();
            bar.on_select(move |index, _window, cx| {
                let _ = this_select.update(cx, |view, cx| {
                    view.state.active_tab = index;
                    cx.notify();
                });
                // Defer sync_tab_bar to avoid re-entrant borrow on TabBar,
                // which is still mutably borrowed by the on_click listener.
                let this_deferred = this_select.clone();
                cx.defer(move |cx| {
                    let _ = this_deferred.update(cx, |view, cx| {
                        view.sync_tab_bar(cx);
                    });
                });
            });

            let this_reorder = this.clone();
            bar.on_reorder(move |from, to, _window, cx| {
                let _ = this_reorder.update(cx, |view, cx| {
                    view.reorder_repo(from, to, cx);
                });
            });

            bar.on_close(move |index, _window, cx| {
                let _ = this.update(cx, |view, cx| {
                    view.remove_repo(index, cx);
                });
            });
        });
    }

    fn sync_tab_bar(&mut self, cx: &mut Context<Self>) {
        let tabs: Vec<TabInfo> = self
            .state
            .repos
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let is_dirty = dd_git::Repository::open(&tab.path)
                    .map(|r| r.is_dirty().unwrap_or(false))
                    .unwrap_or(false);
                TabInfo {
                    name: tab.name.clone(),
                    is_active: i == self.state.active_tab,
                    is_dirty,
                }
            })
            .collect();

        self.tab_bar.update(cx, |bar: &mut TabBar, cx| {
            bar.set_tabs(tabs, cx);
        });
    }

    pub fn open_repository_dialog(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open Git Repository".into()),
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.try_add_repo(path, cx);
                        })
                    });
                }
            }
        })
        .detach();
    }

    pub fn try_add_repo(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.state.repos.iter().any(|r| r.path == path) {
            return;
        }

        match dd_git::Repository::open(&path) {
            Ok(_) => {
                self.error_message = None;
                self.state.add_repo(path.clone());
                let repo_view = cx.new(|cx| RepoView::new(path, cx));
                self.repo_views.push(repo_view);
                self.sync_tab_bar(cx);
                cx.notify();
            }
            Err(_) => {
                self.error_message = Some(format!("{} is not a git repository", path.display()));
                cx.notify();
            }
        }
    }

    pub fn reorder_repo(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        let len = self.repo_views.len();
        if from == to || from >= len || to >= len {
            return;
        }
        let view = self.repo_views.remove(from);
        self.repo_views.insert(to, view);
        self.state.reorder_repos(from, to);
        cx.notify();
        let entity = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = entity.update(cx, |view, cx| {
                view.sync_tab_bar(cx);
            });
        });
    }

    pub fn remove_repo(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.repo_views.len() {
            self.repo_views.remove(index);
            self.state.remove_repo(index);
            cx.notify();
            // Defer sync_tab_bar to avoid re-entrant borrow when called
            // from within a TabBar callback.
            let entity = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = entity.update(cx, |view, cx| {
                    view.sync_tab_bar(cx);
                });
            });
        }
    }

    fn render_welcome(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let error = self.error_message.clone();

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(gpui::div().text_xl().child("DD Merge"))
            .child(
                gpui::div()
                    .text_color(cx.theme().muted_foreground)
                    .child("Open a git repository to get started"),
            )
            .child(
                Button::new("open-repo")
                    .label("Open Repository")
                    .on_click(cx.listener(|view, _event, _window, cx| {
                        view.open_repository_dialog(cx);
                    })),
            )
            .children(error.map(|msg| gpui::div().text_color(gpui::red()).child(msg)))
    }

    pub fn set_active_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.state.repos.len() {
            self.state.active_tab = index;
            self.sync_tab_bar(cx);
            cx.notify();
        }
    }

    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        if !self.state.repos.is_empty() {
            let index = self.state.active_tab.min(self.state.repos.len() - 1);
            self.remove_repo(index, cx);
        }
    }

    pub fn next_tab(&mut self, cx: &mut Context<Self>) {
        let len = self.state.repos.len();
        if len > 1 {
            self.state.active_tab = (self.state.active_tab + 1) % len;
            self.sync_tab_bar(cx);
            cx.notify();
        }
    }

    pub fn previous_tab(&mut self, cx: &mut Context<Self>) {
        let len = self.state.repos.len();
        if len > 1 {
            self.state.active_tab = (self.state.active_tab + len - 1) % len;
            self.sync_tab_bar(cx);
            cx.notify();
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = if self.state.repos.is_empty() {
            self.render_welcome(cx).into_any_element()
        } else {
            let active = self
                .state
                .active_tab
                .min(self.repo_views.len().saturating_sub(1));
            if let Some(repo_view) = self.repo_views.get(active) {
                repo_view.clone().into_any_element()
            } else {
                self.render_welcome(cx).into_any_element()
            }
        };

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.tab_bar.clone())
            .child(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{init_test_repo, init_test_theme};
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_fresh_start_has_no_repos(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .read_with(cx, |view, _cx| {
                assert!(view.state().repos.is_empty());
                assert_eq!(view.repo_view_count(), 0);
                assert!(view.error_message().is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_add_valid_repo(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir.path().to_path_buf(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 1);
                assert_eq!(view.repo_view_count(), 1);
                assert!(view.error_message().is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_add_invalid_path_shows_error(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir = tempfile::TempDir::new().unwrap(); // not a git repo
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir.path().to_path_buf(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(view.error_message().is_some());
                assert!(view.state().repos.is_empty());
                assert_eq!(view.repo_view_count(), 0);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_add_multiple_repos(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 2);
                assert_eq!(view.repo_view_count(), 2);
                assert_eq!(view.state().active_tab, 1); // last added is active
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_remove_repo(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
                view.remove_repo(0, cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 1);
                assert_eq!(view.repo_view_count(), 1);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_tab_switching(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
            })
            .unwrap();

        // After adding 2 repos, active tab should be 1 (last added)
        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().active_tab, 1);
            })
            .unwrap();

        // Switch to tab 0
        window
            .update(cx, |view, _window, cx| {
                view.set_active_tab(0, cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().active_tab, 0);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_add_duplicate_repo_is_ignored(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir.path().to_path_buf(), cx);
                view.try_add_repo(dir.path().to_path_buf(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 1);
                assert_eq!(view.repo_view_count(), 1);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_set_active_tab_out_of_bounds_ignored(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
            })
            .unwrap();

        // active_tab is 1 after adding 2 repos
        window
            .update(cx, |view, _window, cx| {
                view.set_active_tab(99, cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().active_tab, 1);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_remove_repo_out_of_bounds_ignored(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
            })
            .unwrap();

        window
            .update(cx, |view, _window, cx| {
                view.remove_repo(99, cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 2);
                assert_eq!(view.repo_view_count(), 2);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_reorder_repo(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let dir3 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        let name1 = dir1
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
                view.try_add_repo(dir3.path().to_path_buf(), cx);
                view.set_active_tab(0, cx);
                view.reorder_repo(0, 2, cx);
            })
            .unwrap();

        cx.run_until_parked();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 3);
                assert_eq!(view.repo_view_count(), 3);
                // repo1 moved from 0 to 2
                assert_eq!(view.state().repos[2].name, name1);
                // active_tab follows repo1
                assert_eq!(view.state().active_tab, 2);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_tab_bar_reorder_does_not_crash(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let dir3 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
                view.try_add_repo(dir3.path().to_path_buf(), cx);
            })
            .unwrap();

        let tab_bar = window
            .read_with(cx, |view, _cx| view.tab_bar().clone())
            .unwrap();

        let any_handle = window.into();
        cx.update_window(any_handle, |_root, window, app| {
            tab_bar.update(app, |bar, cx| {
                bar.reorder_tab(0, 2, window, cx);
            });
        })
        .unwrap();

        cx.run_until_parked();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 3);
                assert_eq!(view.repo_view_count(), 3);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_tab_bar_select_does_not_crash(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
            })
            .unwrap();

        // Grab the tab_bar entity without holding an AppView borrow,
        // so we can simulate a real tab click (which only borrows TabBar).
        let tab_bar = window
            .read_with(cx, |view, _cx| view.tab_bar().clone())
            .unwrap();

        // Active tab is 1 (last added). Click tab 0 through the TabBar callback,
        // which previously caused a re-entrant borrow panic on TabBar.
        // Use update_window to get Window + App without borrowing AppView.
        let any_handle = window.into();
        cx.update_window(any_handle, |_root, window, app| {
            tab_bar.update(app, |bar, cx| {
                bar.select_tab(0, window, cx);
            });
        })
        .unwrap();

        // Flush deferred effects (sync_tab_bar runs via cx.defer)
        cx.run_until_parked();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().active_tab, 0);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_tab_bar_close_does_not_crash(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir1 = init_test_repo();
        let dir2 = init_test_repo();
        let window = cx.add_window(|window, cx| AppView::new(window, cx));

        window
            .update(cx, |view, _window, cx| {
                view.try_add_repo(dir1.path().to_path_buf(), cx);
                view.try_add_repo(dir2.path().to_path_buf(), cx);
            })
            .unwrap();

        let tab_bar = window
            .read_with(cx, |view, _cx| view.tab_bar().clone())
            .unwrap();

        // Close tab 0 through the TabBar callback,
        // which previously caused a re-entrant borrow panic on TabBar.
        let any_handle = window.into();
        cx.update_window(any_handle, |_root, window, app| {
            tab_bar.update(app, |bar, cx| {
                bar.close_tab(0, window, cx);
            });
        })
        .unwrap();

        cx.run_until_parked();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.state().repos.len(), 1);
                assert_eq!(view.repo_view_count(), 1);
            })
            .unwrap();
    }
}
