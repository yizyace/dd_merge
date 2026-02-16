use std::ops::Range;
use std::path::Path;
use std::sync::LazyLock;

use gpui::Hsla;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

/// A byte-range highlight produced by syntax highlighting.
#[derive(Debug, Clone)]
pub struct SyntaxHighlight {
    /// Byte range into the original line.
    pub range: Range<usize>,
    pub color: Hsla,
}

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Highlight a single line of code, returning byte-range highlights.
/// Falls back to a single range covering the entire line with `fallback_color`
/// if the language is unknown or highlighting fails.
pub fn highlight_line(
    file_path: &str,
    line: &str,
    fallback_color: Hsla,
    is_dark: bool,
) -> Vec<SyntaxHighlight> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let syntax = SYNTAX_SET
        .find_syntax_by_extension(ext)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme_name = if is_dark {
        "base16-ocean.dark"
    } else {
        "base16-ocean.light"
    };
    let theme = &THEME_SET.themes[theme_name];
    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);

    // Append a newline because syntect expects newline-terminated lines
    let input = format!("{}\n", line);
    let Ok(ranges) = highlighter.highlight_line(&input, &SYNTAX_SET) else {
        return vec![SyntaxHighlight {
            range: 0..line.len(),
            color: fallback_color,
        }];
    };

    let mut result = Vec::new();
    let mut offset = 0usize;
    for (style, text) in &ranges {
        let end = offset + text.len();
        // Clamp to original line length (exclude the trailing newline we added)
        let clamped_end = end.min(line.len());
        if offset < clamped_end {
            result.push(SyntaxHighlight {
                range: offset..clamped_end,
                color: style_to_hsla(*style),
            });
        }
        offset = end;
    }

    result
}

fn style_to_hsla(style: Style) -> Hsla {
    let r = style.foreground.r as f32 / 255.0;
    let g = style.foreground.g as f32 / 255.0;
    let b = style.foreground.b as f32 / 255.0;
    let a = style.foreground.a as f32 / 255.0;
    rgb_to_hsla(r, g, b, a)
}

fn rgb_to_hsla(r: f32, g: f32, b: f32, a: f32) -> Hsla {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return Hsla {
            h: 0.0,
            s: 0.0,
            l,
            a,
        };
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h / 6.0
    } else if (max - g).abs() < f32::EPSILON {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    Hsla { h, s, l, a }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust_line() {
        let line = "let x = 42;";
        let highlights = highlight_line("test.rs", line, Hsla::default(), true);
        assert!(!highlights.is_empty());
        // Ranges should cover the entire line without gaps
        let combined: String = highlights.iter().map(|h| &line[h.range.clone()]).collect();
        assert_eq!(combined, line);
    }

    #[test]
    fn test_highlight_unknown_extension() {
        let fallback = Hsla {
            h: 0.5,
            s: 0.5,
            l: 0.5,
            a: 1.0,
        };
        let line = "hello world";
        let highlights = highlight_line("test.zzz_unknown", line, fallback, true);
        assert!(!highlights.is_empty());
        let combined: String = highlights.iter().map(|h| &line[h.range.clone()]).collect();
        assert_eq!(combined, line);
    }

    #[test]
    fn test_highlight_produces_multiple_spans_for_code() {
        let line = "fn main() { println!(\"hello\"); }";
        let highlights = highlight_line("test.rs", line, Hsla::default(), true);
        assert!(
            highlights.len() > 1,
            "expected multiple syntax highlights, got {}: {:?}",
            highlights.len(),
            highlights
        );
    }

    #[test]
    fn test_rgb_to_hsla_white() {
        let c = rgb_to_hsla(1.0, 1.0, 1.0, 1.0);
        assert!((c.l - 1.0).abs() < 0.01);
        assert!(c.s.abs() < 0.01);
    }

    #[test]
    fn test_rgb_to_hsla_pure_red() {
        let c = rgb_to_hsla(1.0, 0.0, 0.0, 1.0);
        assert!(c.h.abs() < 0.01); // hue ~0
        assert!((c.s - 1.0).abs() < 0.01);
    }
}
