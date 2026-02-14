use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{actions, Context, PathPromptOptions, Window};
use gpui_component::{button::Button, v_flex, ActiveTheme};

use dd_core::{AppState, Session};

actions!(dd_merge, [OpenRepository]);

pub struct AppView {
    state: AppState,
    error_message: Option<String>,
}

impl AppView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let state = Session::load().ok().flatten().unwrap_or_default();

        Self {
            state,
            error_message: None,
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    fn open_repository(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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
                self.state.add_repo(path);
                cx.notify();
            }
            Err(_) => {
                self.error_message = Some(format!("{} is not a git repository", path.display()));
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

    fn render_repo_placeholder(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.state.active_tab;
        let repo_name = self
            .state
            .repos
            .get(active)
            .map(|r| r.name.clone())
            .unwrap_or_default();

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(
                gpui::div()
                    .text_xl()
                    .child(format!("Repository: {}", repo_name)),
            )
            .child(
                gpui::div()
                    .text_color(cx.theme().muted_foreground)
                    .child("Full 3-column layout coming soon"),
            )
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = if self.state.repos.is_empty() {
            self.render_welcome(cx).into_any_element()
        } else {
            self.render_repo_placeholder(cx).into_any_element()
        };

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(content)
    }
}
