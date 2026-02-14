use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{actions, Context, Entity, PathPromptOptions, Window};
use gpui_component::{button::Button, v_flex, ActiveTheme};

use dd_core::{AppState, Session};

use crate::repo_view::RepoView;

actions!(dd_merge, [OpenRepository]);

pub struct AppView {
    state: AppState,
    repo_views: Vec<Entity<RepoView>>,
    error_message: Option<String>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = Session::load().ok().flatten().unwrap_or_default();

        let repo_views: Vec<_> = state
            .repos
            .iter()
            .map(|tab| {
                let path = tab.path.clone();
                cx.new(|cx| RepoView::new(path, window, cx))
            })
            .collect();

        Self {
            state,
            repo_views,
            error_message: None,
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn open_repository(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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

    fn try_add_repo(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match dd_git::Repository::open(&path) {
            Ok(_) => {
                self.error_message = None;
                self.state.add_repo(path.clone());
                let repo_view = cx.new(|cx| RepoView::new_without_window(path, cx));
                self.repo_views.push(repo_view);
                cx.notify();
            }
            Err(_) => {
                self.error_message =
                    Some(format!("{} is not a git repository", path.display()));
                cx.notify();
            }
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
                    .on_click(cx.listener(|view, _event, window, cx| {
                        view.open_repository(window, cx);
                    })),
            )
            .children(error.map(|msg| gpui::div().text_color(gpui::red()).child(msg)))
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
            .child(content)
    }
}
