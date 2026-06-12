//! Syntax highlighting for Markdown text via `syntect`.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::settings::ThemeMode;

/// Global syntect state (syntax definitions + themes), initialised once.
struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

static HIGHLIGHTER: OnceLock<Highlighter> = OnceLock::new();

/// Get or initialise the global highlighter.
fn get() -> &'static Highlighter {
    HIGHLIGHTER.get_or_init(|| Highlighter {
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme_set: ThemeSet::load_defaults(),
    })
}

/// Build an egui `LayoutJob` with syntax-highlighted markdown content.
///
/// # Parameters
/// * `text` - Raw markdown text to highlight
/// * `font_size` - Monospace font size in points
/// * `theme_mode` - Dark or Light theme for selecting the syntect colour scheme
/// * `default_color` - Fallback colour for unstyled text
///
/// # Returns
/// An egui `LayoutJob` ready to be laid out by the text renderer
pub fn layout_markdown(
    text: &str,
    font_size: f32,
    theme_mode: ThemeMode,
    default_color: Color32,
) -> LayoutJob {
    let hl = get();
    let theme_name = match theme_mode {
        ThemeMode::Dark => "base16-mocha.dark",
        ThemeMode::Light => "InspiredGitHub",
    };
    let theme = hl
        .theme_set
        .themes
        .get(theme_name)
        .or_else(|| hl.theme_set.themes.values().next())
        .expect("at least one syntect theme present");
    let syntax = hl
        .syntax_set
        .find_syntax_by_extension("md")
        .or_else(|| hl.syntax_set.find_syntax_by_name("Markdown"))
        .unwrap_or_else(|| hl.syntax_set.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, theme);
    let mut job = LayoutJob::default();
    let font = FontId::new(font_size, FontFamily::Monospace);

    for line in LinesWithEndings::from(text) {
        let regions = h.highlight_line(line, &hl.syntax_set).unwrap_or_default();
        if regions.is_empty() {
            job.append(
                line,
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: default_color,
                    ..Default::default()
                },
            );
            continue;
        }
        for (style, segment) in regions {
            job.append(
                segment,
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: convert_color(style),
                    ..Default::default()
                },
            );
        }
    }

    job
}

/// Convert a syntect `Style` foreground colour to egui `Color32`.
fn convert_color(style: Style) -> Color32 {
    let c = style.foreground;
    Color32::from_rgba_premultiplied(c.r, c.g, c.b, c.a)
}

/// Force the highlighter to initialise during startup so the ~700 ms
/// `SyntaxSet::load_defaults_newlines()` cost doesn't hit the first editor frame.
///
/// Also triggers lazy regex compilation inside syntect by creating a
/// `HighlightLines` instance for Markdown and processing one dummy line.
pub fn warmup() {
    let hl = get();
    // Look up Markdown syntax and a theme (the defaults used by the editor).
    let syntax = hl
        .syntax_set
        .find_syntax_by_extension("md")
        .or_else(|| hl.syntax_set.find_syntax_by_name("Markdown"))
        .unwrap_or_else(|| hl.syntax_set.find_syntax_plain_text());
    let theme = hl
        .theme_set
        .themes
        .get("base16-mocha.dark")
        .or_else(|| hl.theme_set.themes.values().next())
        .expect("at least one syntect theme present");
    // Highlight a rich sample to force syntect to compile all common
    // Markdown context regexes (headings, bold, italic, code, lists,
    // blockquotes, links, tables, horizontal rules, etc.) during startup
    // instead of lazily on the first editor frame.
    let mut h = HighlightLines::new(syntax, theme);
    let sample = "# heading\n## subheading\n**bold** *italic* `code`\n- list item\n1. numbered\n> blockquote\n[link](url)\n| a | b |\n|---|---|\n| 1 | 2 |\n---\n";
    for line in sample.lines() {
        let _ = h.highlight_line(line, &hl.syntax_set);
    }
}
