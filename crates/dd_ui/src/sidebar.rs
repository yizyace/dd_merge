use std::collections::HashMap;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{ease_in_out, Animation, AnimationExt, Context, Window};
use gpui_component::{h_flex, scroll::ScrollableElement, v_flex, ActiveTheme};

use dd_git::{BranchInfo, RemoteInfo, StashInfo, TagInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarGroup {
    Branches,
    Remotes,
    Tags,
    Stashes,
    Submodules,
}

pub struct SidebarData {
    pub branches: Vec<BranchInfo>,
    pub remotes: Vec<RemoteInfo>,
    pub tags: Vec<TagInfo>,
    pub stashes: Vec<StashInfo>,
}

impl SidebarData {
    pub fn empty() -> Self {
        Self {
            branches: Vec::new(),
            remotes: Vec::new(),
            tags: Vec::new(),
            stashes: Vec::new(),
        }
    }
}

pub struct Sidebar {
    data: SidebarData,
    collapsed: HashMap<SidebarGroup, bool>,
}

impl Sidebar {
    pub fn new_empty() -> Self {
        Self {
            data: SidebarData::empty(),
            collapsed: HashMap::new(),
        }
    }

    pub fn toggle_group(&mut self, group: SidebarGroup, cx: &mut Context<Self>) {
        let entry = self.collapsed.entry(group).or_insert(false);
        *entry = !*entry;
        cx.notify();
    }

    pub fn is_collapsed(&self, group: SidebarGroup) -> bool {
        self.collapsed.get(&group).copied().unwrap_or(false)
    }

    pub fn data(&self) -> &SidebarData {
        &self.data
    }

    pub fn set_data(&mut self, data: SidebarData, cx: &mut Context<Self>) {
        self.data = data;
        cx.notify();
    }

    fn render_section(
        &self,
        group: SidebarGroup,
        title: &str,
        count: usize,
        items: Vec<impl IntoElement>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let collapsed = self.is_collapsed(group);
        let arrow = if collapsed { "▶" } else { "▼" };

        v_flex()
            .w_full()
            .gap_0p5()
            .child(
                h_flex()
                    .id(gpui::ElementId::Name(title.to_string().into()))
                    .px_2()
                    .py_1()
                    .gap_1()
                    .cursor_pointer()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .on_click(cx.listener(move |view, _event, _window, cx| {
                        view.toggle_group(group, cx);
                    }))
                    .child(arrow)
                    .child(format!("{} ({})", title, count)),
            )
            .child({
                let target_h = count as f32 * 28.0;
                let anim_id = if collapsed {
                    format!("collapse-{}", title)
                } else {
                    format!("expand-{}", title)
                };
                v_flex()
                    .w_full()
                    .overflow_hidden()
                    .children(items)
                    .with_animation(
                        gpui::ElementId::Name(anim_id.into()),
                        Animation::new(Duration::from_millis(150)).with_easing(ease_in_out),
                        move |el, delta| {
                            let h = if collapsed {
                                (1.0 - delta) * target_h
                            } else {
                                delta * target_h
                            };
                            el.max_h(gpui::px(h))
                        },
                    )
            })
    }

    fn render_item(&self, label: String, is_active: bool, cx: &Context<Self>) -> impl IntoElement {
        gpui::div()
            .px_3()
            .py_0p5()
            .text_sm()
            .w_full()
            .text_color(if is_active {
                cx.theme().foreground
            } else {
                cx.theme().muted_foreground
            })
            .when(is_active, |el| el.font_weight(gpui::FontWeight::BOLD))
            .child(label)
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let branch_items: Vec<_> = self
            .data
            .branches
            .iter()
            .map(|b| self.render_item(b.name.clone(), b.is_head, cx))
            .collect();

        let remote_items: Vec<_> = self
            .data
            .remotes
            .iter()
            .map(|r| self.render_item(r.name.clone(), false, cx))
            .collect();

        let tag_items: Vec<_> = self
            .data
            .tags
            .iter()
            .map(|t| self.render_item(t.name.clone(), false, cx))
            .collect();

        let stash_items: Vec<_> = self
            .data
            .stashes
            .iter()
            .map(|s| self.render_item(s.message.clone(), false, cx))
            .collect();

        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .py_2()
            .gap_2()
            .overflow_y_scrollbar()
            .child(self.render_section(
                SidebarGroup::Branches,
                "BRANCHES",
                self.data.branches.len(),
                branch_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Remotes,
                "REMOTES",
                self.data.remotes.len(),
                remote_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Tags,
                "TAGS",
                self.data.tags.len(),
                tag_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Stashes,
                "STASHES",
                self.data.stashes.len(),
                stash_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Submodules,
                "SUBMODULES",
                0,
                Vec::<gpui::AnyElement>::new(),
                cx,
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn test_set_data_updates_sidebar(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| Sidebar::new_empty());

        window
            .read_with(cx, |view, _cx| {
                assert!(view.data().branches.is_empty());
            })
            .unwrap();

        window
            .update(cx, |view, _window, cx| {
                view.set_data(
                    SidebarData {
                        branches: vec![BranchInfo {
                            name: "main".into(),
                            is_head: true,
                        }],
                        remotes: vec![RemoteInfo {
                            name: "origin".into(),
                        }],
                        tags: vec![],
                        stashes: vec![],
                    },
                    cx,
                );
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.data().branches.len(), 1);
                assert_eq!(view.data().branches[0].name, "main");
                assert_eq!(view.data().remotes.len(), 1);
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_toggle_group_collapses_and_expands(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| Sidebar::new_empty());

        // All groups start expanded
        window
            .read_with(cx, |view, _cx| {
                assert!(!view.is_collapsed(SidebarGroup::Branches));
                assert!(!view.is_collapsed(SidebarGroup::Remotes));
                assert!(!view.is_collapsed(SidebarGroup::Tags));
            })
            .unwrap();

        // Toggle Branches → collapsed
        window
            .update(cx, |view, _window, cx| {
                view.toggle_group(SidebarGroup::Branches, cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(view.is_collapsed(SidebarGroup::Branches));
                // Other groups unaffected
                assert!(!view.is_collapsed(SidebarGroup::Remotes));
                assert!(!view.is_collapsed(SidebarGroup::Tags));
            })
            .unwrap();

        // Toggle Branches again → re-expanded
        window
            .update(cx, |view, _window, cx| {
                view.toggle_group(SidebarGroup::Branches, cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(!view.is_collapsed(SidebarGroup::Branches));
            })
            .unwrap();
    }

    #[test]
    fn test_sidebar_data_groups_refs() {
        let data = SidebarData {
            branches: vec![
                BranchInfo {
                    name: "main".into(),
                    is_head: true,
                },
                BranchInfo {
                    name: "feature".into(),
                    is_head: false,
                },
            ],
            remotes: vec![RemoteInfo {
                name: "origin".into(),
            }],
            tags: vec![TagInfo {
                name: "v1.0".into(),
            }],
            stashes: vec![StashInfo {
                message: "WIP".into(),
            }],
        };
        assert_eq!(data.branches.len(), 2);
        assert_eq!(data.remotes.len(), 1);
        assert_eq!(data.tags.len(), 1);
        assert_eq!(data.stashes.len(), 1);
    }
}
