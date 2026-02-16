pub mod app_view;
pub mod commit_list;
pub mod diff_view;
pub mod repo_view;
pub mod sidebar;
pub mod syntax;
pub mod tab_bar;
pub mod theme;

pub use app_view::AppView;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod tests {
    #[test]
    fn test_name() {
        assert_eq!(env!("CARGO_PKG_NAME"), "dd_ui");
    }
}
