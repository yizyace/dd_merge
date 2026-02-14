use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{Context, Entity, Window};
use gpui_component::{h_flex, v_flex, ActiveTheme};

use dd_git::Repository;

use crate::commit_list::CommitList;
use crate::sidebar::{Sidebar, SidebarData};

const COMMIT_LIMIT: usize = 100;

pub struct RepoView {
    path: PathBuf,
    repo_name: String,
    sidebar: Entity<Sidebar>,
    commit_list: Entity<CommitList>,
}

impl RepoView {
    pub fn new(path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_without_window(path, cx)
    }

    pub fn new_without_window(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let repo_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let sidebar = cx.new(|_cx| Sidebar::new_empty());
        let commit_list = cx.new(|_cx| CommitList::new_empty());

        let mut view = Self {
            path,
            repo_name,
            sidebar,
            commit_list,
        };
        view.load_repo_data(cx);
        view
    }

    pub fn repo_name(&self) -> &str {
        &self.repo_name
    }

    pub fn sidebar(&self) -> &Entity<Sidebar> {
        &self.sidebar
    }

    pub fn commit_list(&self) -> &Entity<CommitList> {
        &self.commit_list
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
                // Commits column
                v_flex()
                    .h_full()
                    .flex_1()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(self.commit_list.clone()),
            )
            .child(
                // Diff column placeholder
                v_flex()
                    .h_full()
                    .flex_1()
                    .flex_grow()
                    .p_4()
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Select a commit to view its diff"),
                    ),
            )
    }
}
