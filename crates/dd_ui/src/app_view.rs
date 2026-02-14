use gpui::prelude::*;
use gpui::{Context, Window};
use gpui_component::{ActiveTheme, v_flex};

pub struct AppView;

impl AppView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .items_center()
            .justify_center()
            .child(
                gpui::div()
                    .text_xl()
                    .child("DD Merge"),
            )
    }
}
