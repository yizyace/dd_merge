use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{Context, Entity, Window};
use gpui_component::{h_flex, v_flex, ActiveTheme};

use dd_git::Repository;

use crate::commit_list::CommitList;
use crate::diff_view::DiffView;
use crate::sidebar::{Sidebar, SidebarData};

const COMMIT_LIMIT: usize = 100;

pub struct RepoView {
    path: PathBuf,
    repo_name: String,
    sidebar: Entity<Sidebar>,
    commit_list: Entity<CommitList>,
    diff_view: Entity<DiffView>,
}

impl RepoView {
    pub fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let repo_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let sidebar = cx.new(|_cx| Sidebar::new_empty());
        let commit_list = cx.new(|_cx| CommitList::new_empty());
        let diff_view = cx.new(|_cx| DiffView::new_empty());

        let mut view = Self {
            path,
            repo_name,
            sidebar,
            commit_list,
            diff_view,
        };
        view.load_repo_data(cx);
        view.setup_commit_selection(cx);
        view
    }

    pub fn repo_name(&self) -> &str {
        &self.repo_name
    }

    pub fn commit_list(&self) -> &Entity<CommitList> {
        &self.commit_list
    }

    pub fn diff_view(&self) -> &Entity<DiffView> {
        &self.diff_view
    }

    pub fn sidebar(&self) -> &Entity<Sidebar> {
        &self.sidebar
    }

    fn setup_commit_selection(&mut self, cx: &mut Context<Self>) {
        let diff_view = self.diff_view.clone();
        let repo_path = self.path.clone();

        self.commit_list.update(cx, |list, _cx| {
            list.on_select(
                move |commit, _window, cx| match Repository::open(&repo_path) {
                    Ok(repo) => match repo.diff_commit(&commit.oid) {
                        Ok(diffs) => {
                            diff_view.update(cx, |view, cx| {
                                view.set_diffs(diffs, cx);
                            });
                        }
                        Err(e) => {
                            diff_view.update(cx, |view, cx| {
                                view.set_error(format!("Failed to load diff: {e}"), cx);
                            });
                        }
                    },
                    Err(e) => {
                        diff_view.update(cx, |view, cx| {
                            view.set_error(format!("Failed to open repository: {e}"), cx);
                        });
                    }
                },
            );
        });
    }

    fn load_repo_data(&mut self, cx: &mut Context<Self>) {
        if let Ok(repo) = Repository::open(&self.path) {
            let branches = repo.branches().unwrap_or_default();
            let remotes = repo.remotes().unwrap_or_default();
            let tags = repo.tags().unwrap_or_default();
            let stashes = repo.stashes().unwrap_or_default();

            self.sidebar.update(cx, |sidebar, cx| {
                sidebar.set_data(
                    SidebarData {
                        branches,
                        remotes,
                        tags,
                        stashes,
                    },
                    cx,
                );
            });

            let commits = repo.commits(COMMIT_LIMIT).unwrap_or_default();
            self.commit_list.update(cx, |list, cx| {
                list.set_commits(commits, cx);
            });
        }
    }
}

impl Render for RepoView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .child(self.sidebar.clone())
            .child(
                v_flex()
                    .h_full()
                    .flex_1()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(self.commit_list.clone()),
            )
            .child(
                v_flex()
                    .h_full()
                    .flex_1()
                    .flex_grow()
                    .child(self.diff_view.clone()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{init_test_repo, init_test_repo_with_changes, init_test_theme};
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_repo_view_loads_branches(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir = init_test_repo();
        let path = dir.path().to_path_buf();

        let window = cx.add_window(|_window, cx| RepoView::new(path, cx));

        window
            .read_with(cx, |view, cx| {
                let sidebar = view.sidebar().read(cx);
                let data = sidebar.data();
                assert!(
                    data.branches.iter().any(|b| b.name == "main" && b.is_head),
                    "expected 'main' branch with is_head == true"
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_repo_view_loads_commits(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir = init_test_repo_with_changes();
        let path = dir.path().to_path_buf();

        let window = cx.add_window(|_window, cx| RepoView::new(path, cx));

        window
            .read_with(cx, |view, cx| {
                let commit_list = view.commit_list().read(cx);
                assert!(
                    commit_list.commits().len() >= 2,
                    "expected at least 2 commits, got {}",
                    commit_list.commits().len()
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_commit_selection_loads_diff(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir = init_test_repo_with_changes();
        let path = dir.path().to_path_buf();

        let window = cx.add_window(|_window, cx| RepoView::new(path, cx));

        // Select the first commit (most recent = "second commit")
        window
            .update(cx, |view, window, cx| {
                let cl = view.commit_list().clone();
                cl.update(cx, |list, cx| {
                    list.select_commit(0, window, cx);
                });
            })
            .unwrap();

        // Verify diff was loaded
        window
            .read_with(cx, |view, cx| {
                let diff_view = view.diff_view().read(cx);
                assert!(
                    !diff_view.diffs().is_empty(),
                    "expected non-empty diffs after selecting a commit"
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_repo_name_extracted_from_path(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));
        let dir = init_test_repo();
        let expected_name = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let path = dir.path().to_path_buf();

        let window = cx.add_window(|_window, cx| RepoView::new(path, cx));

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.repo_name(), expected_name);
            })
            .unwrap();
    }
}
