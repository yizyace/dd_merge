pub mod app_view;
pub mod repo_view;
pub mod sidebar;
pub mod theme;

pub use app_view::AppView;

#[cfg(test)]
mod tests {
    #[test]
    fn test_name() {
        assert_eq!(env!("CARGO_PKG_NAME"), "dd_ui");
    }
}
