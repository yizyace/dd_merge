use gpui::prelude::*;
use gpui::{Context, Window};
use gpui_component::{h_flex, ActiveTheme};

pub struct TabInfo {
    pub name: String,
    pub is_active: bool,
}

#[derive(Clone)]
struct DraggedTab {
    index: usize,
    name: String,
}

struct DragPreview {
    name: String,
}

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .px_3()
            .py_1()
            .bg(cx.theme().muted)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .text_sm()
            .child(self.name.clone())
    }
}

pub struct TabBar {
    tabs: Vec<TabInfo>,
    #[allow(clippy::type_complexity)]
    on_select: Option<Box<dyn Fn(usize, &mut Window, &mut Context<Self>) + 'static>>,
    #[allow(clippy::type_complexity)]
    on_close: Option<Box<dyn Fn(usize, &mut Window, &mut Context<Self>) + 'static>>,
    #[allow(clippy::type_complexity)]
    on_reorder: Option<Box<dyn Fn(usize, usize, &mut Window, &mut Context<Self>) + 'static>>,
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            on_select: None,
            on_close: None,
            on_reorder: None,
        }
    }

    pub fn set_tabs(&mut self, tabs: Vec<TabInfo>, cx: &mut Context<Self>) {
        self.tabs = tabs;
        cx.notify();
    }

    pub fn on_select(
        &mut self,
        callback: impl Fn(usize, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.on_select = Some(Box::new(callback));
    }

    pub fn on_close(
        &mut self,
        callback: impl Fn(usize, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.on_close = Some(Box::new(callback));
    }

    pub fn on_reorder(
        &mut self,
        callback: impl Fn(usize, usize, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.on_reorder = Some(Box::new(callback));
    }

    pub fn select_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref on_select) = self.on_select {
            on_select(index, window, cx);
        }
    }

    pub fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref on_close) = self.on_close {
            on_close(index, window, cx);
        }
    }

    pub fn reorder_tab(
        &mut self,
        from: usize,
        to: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ref on_reorder) = self.on_reorder {
            on_reorder(from, to, window, cx);
        }
    }
}

impl Render for TabBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.tabs.is_empty() {
            return gpui::div().into_any_element();
        }

        let tab_elements: Vec<_> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let is_active = tab.is_active;
                let name = tab.name.clone();

                h_flex()
                    .id(gpui::ElementId::Integer(i as u64))
                    .px_3()
                    .py_1()
                    .gap_2()
                    .cursor_pointer()
                    .border_b_2()
                    .when(is_active, |el| el.border_color(cx.theme().primary))
                    .when(!is_active, |el| el.border_color(gpui::transparent_black()))
                    .when(is_active, |el| el.bg(cx.theme().muted))
                    .hover(|el| el.bg(cx.theme().muted))
                    .on_click(cx.listener(move |view, _event, window, cx| {
                        view.select_tab(i, window, cx);
                    }))
                    .on_drag(
                        DraggedTab {
                            index: i,
                            name: name.clone(),
                        },
                        |dragged, _, _, cx| {
                            cx.new(|_| DragPreview {
                                name: dragged.name.clone(),
                            })
                        },
                    )
                    .on_drop(cx.listener(move |view, dragged: &DraggedTab, window, cx| {
                        view.reorder_tab(dragged.index, i, window, cx);
                    }))
                    .drag_over::<DraggedTab>(|style, _, _, _| {
                        style.bg(gpui::hsla(0.6, 0.3, 0.5, 0.15))
                    })
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(if is_active {
                                cx.theme().foreground
                            } else {
                                cx.theme().muted_foreground
                            })
                            .child(name),
                    )
                    .child(
                        gpui::div()
                            .id(gpui::ElementId::Integer(1000 + i as u64))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .cursor_pointer()
                            .hover(|el| el.text_color(cx.theme().foreground))
                            .on_click(cx.listener(move |view, _event, window, cx| {
                                view.close_tab(i, window, cx);
                            }))
                            .child("×"),
                    )
            })
            .collect();

        h_flex()
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .overflow_x_hidden()
            .children(tab_elements)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::init_test_theme;
    use gpui::TestAppContext;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn test_tab_info() {
        let tabs = vec![
            TabInfo {
                name: "repo1".into(),
                is_active: true,
            },
            TabInfo {
                name: "repo2".into(),
                is_active: false,
            },
        ];
        assert_eq!(tabs.len(), 2);
        assert!(tabs[0].is_active);
        assert!(!tabs[1].is_active);
    }

    #[gpui::test]
    fn test_select_tab_fires_callback(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));

        let selected = Rc::new(Cell::new(None::<usize>));
        let selected_clone = selected.clone();

        let window = cx.add_window(|_window, _cx| TabBar::new());

        window
            .update(cx, |bar, _window, cx| {
                bar.set_tabs(
                    vec![
                        TabInfo {
                            name: "repo1".into(),
                            is_active: true,
                        },
                        TabInfo {
                            name: "repo2".into(),
                            is_active: false,
                        },
                    ],
                    cx,
                );
                bar.on_select(move |index, _window, _cx| {
                    selected_clone.set(Some(index));
                });
            })
            .unwrap();

        window
            .update(cx, |bar, window, cx| {
                bar.select_tab(1, window, cx);
            })
            .unwrap();

        assert_eq!(selected.get(), Some(1));
    }

    #[gpui::test]
    fn test_close_tab_fires_callback(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));

        let closed = Rc::new(Cell::new(None::<usize>));
        let closed_clone = closed.clone();

        let window = cx.add_window(|_window, _cx| TabBar::new());

        window
            .update(cx, |bar, _window, cx| {
                bar.set_tabs(
                    vec![
                        TabInfo {
                            name: "repo1".into(),
                            is_active: true,
                        },
                        TabInfo {
                            name: "repo2".into(),
                            is_active: false,
                        },
                    ],
                    cx,
                );
                bar.on_close(move |index, _window, _cx| {
                    closed_clone.set(Some(index));
                });
            })
            .unwrap();

        window
            .update(cx, |bar, window, cx| {
                bar.close_tab(0, window, cx);
            })
            .unwrap();

        assert_eq!(closed.get(), Some(0));
    }

    #[gpui::test]
    fn test_reorder_tab_fires_callback(cx: &mut TestAppContext) {
        cx.update(|cx| init_test_theme(cx));

        let reordered = Rc::new(Cell::new(None::<(usize, usize)>));
        let reordered_clone = reordered.clone();

        let window = cx.add_window(|_window, _cx| TabBar::new());

        window
            .update(cx, |bar, _window, cx| {
                bar.set_tabs(
                    vec![
                        TabInfo {
                            name: "repo1".into(),
                            is_active: true,
                        },
                        TabInfo {
                            name: "repo2".into(),
                            is_active: false,
                        },
                        TabInfo {
                            name: "repo3".into(),
                            is_active: false,
                        },
                    ],
                    cx,
                );
                bar.on_reorder(move |from, to, _window, _cx| {
                    reordered_clone.set(Some((from, to)));
                });
            })
            .unwrap();

        window
            .update(cx, |bar, window, cx| {
                bar.reorder_tab(0, 2, window, cx);
            })
            .unwrap();

        assert_eq!(reordered.get(), Some((0, 2)));
    }
}
