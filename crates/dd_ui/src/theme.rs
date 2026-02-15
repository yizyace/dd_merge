use gpui::App;
use gpui_component::{Theme, ThemeMode};

pub fn setup_dark_theme(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_mode() {
        let mode = ThemeMode::Dark;
        assert!(mode.is_dark());
    }
}
