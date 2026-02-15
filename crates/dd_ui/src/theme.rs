use std::any::Any;

use gpui::{App, Context, Hsla};
use gpui_component::{ActiveTheme, Theme, ThemeMode};

pub fn setup_dark_theme(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
}

pub struct DiffTheme {
    pub add_bg: Hsla,
    pub add_highlight_bg: Hsla,
    pub del_bg: Hsla,
    pub del_highlight_bg: Hsla,
    pub ctx_bg: Hsla,
    pub line_number_fg: Hsla,
    pub ctx_fg: Hsla,
}

impl DiffTheme {
    pub fn from_cx(cx: &Context<impl Any>) -> Self {
        let theme = cx.theme();
        let success_h = theme.success.h;
        let danger_h = theme.danger.h;

        let is_dark = theme.background.l < 0.5;
        let (bg_l, hl_l) = if is_dark { (0.10, 0.28) } else { (0.92, 0.78) };

        Self {
            add_bg: Hsla {
                h: success_h,
                s: 0.30,
                l: bg_l,
                a: 1.0,
            },
            add_highlight_bg: Hsla {
                h: success_h,
                s: 0.55,
                l: hl_l,
                a: 1.0,
            },
            del_bg: Hsla {
                h: danger_h,
                s: 0.30,
                l: bg_l,
                a: 1.0,
            },
            del_highlight_bg: Hsla {
                h: danger_h,
                s: 0.55,
                l: hl_l,
                a: 1.0,
            },
            ctx_bg: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.0,
            },
            line_number_fg: theme.muted_foreground,
            ctx_fg: theme.muted_foreground,
        }
    }
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
