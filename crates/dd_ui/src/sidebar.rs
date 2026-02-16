use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{ease_in_out, Animation, AnimationExt, ClickEvent, Context, Window};
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

#[derive(Debug, Clone)]
struct BranchTreeNode {
    segment: String,
    path: String,
    branch: Option<BranchInfo>,
    children: Vec<BranchTreeNode>,
}

impl BranchTreeNode {
    fn build(branches: &[BranchInfo]) -> Vec<BranchTreeNode> {
        let mut roots: Vec<BranchTreeNode> = Vec::new();

        for branch in branches {
            let segments: Vec<&str> = branch.name.split('/').collect();
            Self::insert(&mut roots, &segments, 0, branch);
        }

        Self::sort_all(&mut roots);
        roots
    }

    fn insert(
        nodes: &mut Vec<BranchTreeNode>,
        segments: &[&str],
        depth: usize,
        branch: &BranchInfo,
    ) {
        if depth >= segments.len() {
            return;
        }

        let segment = segments[depth];
        let path = segments[..=depth].join("/");
        let is_last = depth == segments.len() - 1;

        let existing = nodes
            .iter_mut()
            .find(|n| n.segment == segment && n.path == path);

        if let Some(node) = existing {
            if is_last {
                node.branch = Some(branch.clone());
            } else {
                Self::insert(&mut node.children, segments, depth + 1, branch);
            }
        } else {
            let mut node = BranchTreeNode {
                segment: segment.to_string(),
                path,
                branch: if is_last { Some(branch.clone()) } else { None },
                children: Vec::new(),
            };
            if !is_last {
                Self::insert(&mut node.children, segments, depth + 1, branch);
            }
            nodes.push(node);
        }
    }

    fn sort_all(nodes: &mut [BranchTreeNode]) {
        nodes.sort_by(|a, b| a.segment.cmp(&b.segment));
        for node in nodes.iter_mut() {
            Self::sort_all(&mut node.children);
        }
    }

    fn visible_count(&self, collapsed: &HashSet<String>) -> usize {
        let mut count = 1; // this node itself
        if !self.children.is_empty() && !collapsed.contains(&self.path) {
            for child in &self.children {
                count += child.visible_count(collapsed);
            }
        }
        count
    }
}

pub struct Sidebar {
    data: SidebarData,
    collapsed: HashMap<SidebarGroup, bool>,
    branch_tree: Vec<BranchTreeNode>,
    collapsed_folders: HashSet<String>,
    #[allow(clippy::type_complexity)]
    on_branch_checkout: Option<Box<dyn Fn(&BranchInfo, &mut Window, &mut Context<Self>) + 'static>>,
}

impl Sidebar {
    pub fn new_empty() -> Self {
        Self {
            data: SidebarData::empty(),
            collapsed: HashMap::new(),
            branch_tree: Vec::new(),
            collapsed_folders: HashSet::new(),
            on_branch_checkout: None,
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
        self.branch_tree = BranchTreeNode::build(&data.branches);
        self.data = data;
        cx.notify();
    }

    pub fn toggle_folder(&mut self, path: String, cx: &mut Context<Self>) {
        if self.collapsed_folders.contains(&path) {
            self.collapsed_folders.remove(&path);
        } else {
            self.collapsed_folders.insert(path);
        }
        cx.notify();
    }

    pub fn is_folder_collapsed(&self, path: &str) -> bool {
        self.collapsed_folders.contains(path)
    }

    pub fn on_branch_checkout(
        &mut self,
        callback: impl Fn(&BranchInfo, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.on_branch_checkout = Some(Box::new(callback));
    }

    fn render_section(
        &self,
        group: SidebarGroup,
        title: &str,
        display_count: usize,
        visible_count: usize,
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
                    .child(format!("{} ({})", title, display_count)),
            )
            .child({
                let target_h = visible_count as f32 * 28.0;
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

    fn render_branch_tree_nodes(
        &self,
        nodes: &[BranchTreeNode],
        depth: usize,
        cx: &Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let mut elements = Vec::new();
        for node in nodes {
            elements.extend(self.render_branch_tree_node(node, depth, cx));
        }
        elements
    }

    fn render_branch_tree_node(
        &self,
        node: &BranchTreeNode,
        depth: usize,
        cx: &Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let mut elements = Vec::new();
        let is_folder = !node.children.is_empty();
        let is_active = node.branch.as_ref().is_some_and(|b| b.is_head);
        let indent = depth as f32 * 12.0;

        if is_folder {
            let collapsed = self.is_folder_collapsed(&node.path);
            let arrow = if collapsed { "▶ " } else { "▼ " };
            let path = node.path.clone();

            elements.push(
                h_flex()
                    .id(gpui::ElementId::Name(
                        format!("folder-{}", node.path).into(),
                    ))
                    .pl(gpui::px(indent + 12.0)) // 12px base + indent
                    .py_0p5()
                    .w_full()
                    .cursor_pointer()
                    .text_sm()
                    .text_color(if is_active {
                        cx.theme().foreground
                    } else {
                        cx.theme().muted_foreground
                    })
                    .when(is_active, |el| el.font_weight(gpui::FontWeight::BOLD))
                    .on_click(cx.listener(move |view, _event, _window, cx| {
                        view.toggle_folder(path.clone(), cx);
                    }))
                    .child(format!("{}{}", arrow, node.segment))
                    .into_any_element(),
            );

            // Always render children (needed for animation)
            let child_elements = self.render_branch_tree_nodes(&node.children, depth + 1, cx);

            let children_visible: usize = node
                .children
                .iter()
                .map(|c| c.visible_count(&self.collapsed_folders))
                .sum();
            let target_h = children_visible as f32 * 28.0;

            let anim_id = if collapsed {
                format!("collapse-folder-{}", node.path)
            } else {
                format!("expand-folder-{}", node.path)
            };

            elements.push(
                v_flex()
                    .w_full()
                    .overflow_hidden()
                    .children(child_elements)
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
                    .into_any_element(),
            );
        } else {
            // Leaf node — no arrow, extra indent to align with folder text
            let branch_info = node.branch.clone().unwrap();
            elements.push(
                gpui::div()
                    .id(gpui::ElementId::Name(
                        format!("branch-{}", node.path).into(),
                    ))
                    .pl(gpui::px(indent + 12.0 + 16.0)) // base + indent + arrow space
                    .py_0p5()
                    .text_sm()
                    .w_full()
                    .cursor_pointer()
                    .text_color(if is_active {
                        cx.theme().foreground
                    } else {
                        cx.theme().muted_foreground
                    })
                    .when(is_active, |el| el.font_weight(gpui::FontWeight::BOLD))
                    .on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                        if let ClickEvent::Mouse(mouse) = event {
                            if mouse.down.click_count == 2 {
                                if let Some(ref on_checkout) = view.on_branch_checkout {
                                    on_checkout(&branch_info, window, cx);
                                }
                            }
                        }
                    }))
                    .child(node.segment.clone())
                    .into_any_element(),
            );
        }

        elements
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
        let branch_display_count = self.data.branches.len();
        let branch_visible_count: usize = self
            .branch_tree
            .iter()
            .map(|n| n.visible_count(&self.collapsed_folders))
            .sum();
        let branch_items = self.render_branch_tree_nodes(&self.branch_tree, 0, cx);

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

        let remote_count = self.data.remotes.len();
        let tag_count = self.data.tags.len();
        let stash_count = self.data.stashes.len();

        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .py_2()
            .gap_2()
            .overflow_y_scrollbar()
            .child(self.render_section(
                SidebarGroup::Branches,
                "BRANCHES",
                branch_display_count,
                branch_visible_count,
                branch_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Remotes,
                "REMOTES",
                remote_count,
                remote_count,
                remote_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Tags,
                "TAGS",
                tag_count,
                tag_count,
                tag_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Stashes,
                "STASHES",
                stash_count,
                stash_count,
                stash_items,
                cx,
            ))
            .child(self.render_section(
                SidebarGroup::Submodules,
                "SUBMODULES",
                0,
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

    #[test]
    fn test_build_tree_flat_branches() {
        let branches = vec![
            BranchInfo {
                name: "main".into(),
                is_head: true,
            },
            BranchInfo {
                name: "develop".into(),
                is_head: false,
            },
        ];
        let tree = BranchTreeNode::build(&branches);
        assert_eq!(tree.len(), 2);
        // Sorted alphabetically
        assert_eq!(tree[0].segment, "develop");
        assert_eq!(tree[1].segment, "main");
        // Both are leaves (no children)
        assert!(tree[0].children.is_empty());
        assert!(tree[1].children.is_empty());
        // Branch info present
        assert!(tree[0].branch.is_some());
        assert!(tree[1].branch.is_some());
        assert!(tree[1].branch.as_ref().unwrap().is_head);
    }

    #[test]
    fn test_build_tree_nested() {
        let branches = vec![BranchInfo {
            name: "checkpoints/260214/feat/mvp-baseline1/1".into(),
            is_head: false,
        }];
        let tree = BranchTreeNode::build(&branches);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].segment, "checkpoints");
        assert!(tree[0].branch.is_none());
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].segment, "260214");
        // Drill down to the leaf
        let leaf = &tree[0].children[0].children[0].children[0].children[0];
        assert_eq!(leaf.segment, "1");
        assert!(leaf.branch.is_some());
        assert!(leaf.children.is_empty());
    }

    #[test]
    fn test_build_tree_shared_prefix() {
        let branches = vec![
            BranchInfo {
                name: "feat/a".into(),
                is_head: false,
            },
            BranchInfo {
                name: "feat/b".into(),
                is_head: false,
            },
        ];
        let tree = BranchTreeNode::build(&branches);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].segment, "feat");
        assert!(tree[0].branch.is_none());
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].segment, "a");
        assert_eq!(tree[0].children[1].segment, "b");
    }

    #[test]
    fn test_visible_count() {
        let branches = vec![
            BranchInfo {
                name: "feat/a".into(),
                is_head: false,
            },
            BranchInfo {
                name: "feat/b".into(),
                is_head: false,
            },
            BranchInfo {
                name: "main".into(),
                is_head: true,
            },
        ];
        let tree = BranchTreeNode::build(&branches);
        let collapsed = HashSet::new();

        // All expanded: feat(1) + a(1) + b(1) + main(1) = 4
        let total: usize = tree.iter().map(|n| n.visible_count(&collapsed)).sum();
        assert_eq!(total, 4);

        // Collapse "feat": feat(1) + main(1) = 2
        let mut collapsed = HashSet::new();
        collapsed.insert("feat".to_string());
        let total: usize = tree.iter().map(|n| n.visible_count(&collapsed)).sum();
        assert_eq!(total, 2);
    }

    #[test]
    fn test_build_tree_branch_and_folder_same_name() {
        // Both "main" (a branch) and "main/hotfix" exist
        let branches = vec![
            BranchInfo {
                name: "main".into(),
                is_head: true,
            },
            BranchInfo {
                name: "main/hotfix".into(),
                is_head: false,
            },
        ];
        let tree = BranchTreeNode::build(&branches);
        assert_eq!(tree.len(), 1);
        // "main" is both folder and branch
        assert!(tree[0].branch.is_some());
        assert!(tree[0].branch.as_ref().unwrap().is_head);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].segment, "hotfix");
    }

    #[gpui::test]
    fn test_toggle_folder(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| Sidebar::new_empty());

        // Folders start expanded
        window
            .read_with(cx, |view, _cx| {
                assert!(!view.is_folder_collapsed("feat"));
            })
            .unwrap();

        // Toggle → collapsed
        window
            .update(cx, |view, _window, cx| {
                view.toggle_folder("feat".to_string(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(view.is_folder_collapsed("feat"));
            })
            .unwrap();

        // Toggle again → expanded
        window
            .update(cx, |view, _window, cx| {
                view.toggle_folder("feat".to_string(), cx);
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert!(!view.is_folder_collapsed("feat"));
            })
            .unwrap();
    }

    #[gpui::test]
    fn test_set_data_rebuilds_tree(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| crate::test_helpers::init_test_theme(cx));
        let window = cx.add_window(|_window, _cx| Sidebar::new_empty());

        // Initially empty tree
        window
            .read_with(cx, |view, _cx| {
                assert!(view.branch_tree.is_empty());
            })
            .unwrap();

        // Set data with nested branches
        window
            .update(cx, |view, _window, cx| {
                view.set_data(
                    SidebarData {
                        branches: vec![
                            BranchInfo {
                                name: "feat/a".into(),
                                is_head: false,
                            },
                            BranchInfo {
                                name: "feat/b".into(),
                                is_head: false,
                            },
                        ],
                        remotes: vec![],
                        tags: vec![],
                        stashes: vec![],
                    },
                    cx,
                );
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.branch_tree.len(), 1);
                assert_eq!(view.branch_tree[0].segment, "feat");
                assert_eq!(view.branch_tree[0].children.len(), 2);
            })
            .unwrap();

        // Update data → tree is rebuilt
        window
            .update(cx, |view, _window, cx| {
                view.set_data(
                    SidebarData {
                        branches: vec![BranchInfo {
                            name: "main".into(),
                            is_head: true,
                        }],
                        remotes: vec![],
                        tags: vec![],
                        stashes: vec![],
                    },
                    cx,
                );
            })
            .unwrap();

        window
            .read_with(cx, |view, _cx| {
                assert_eq!(view.branch_tree.len(), 1);
                assert_eq!(view.branch_tree[0].segment, "main");
                assert!(view.branch_tree[0].children.is_empty());
            })
            .unwrap();
    }
}
