use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::settings::ThemeMode;

struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

static HIGHLIGHTER: OnceLock<Highlighter> = OnceLock::new();

fn get() -> &'static Highlighter {
    HIGHLIGHTER.get_or_init(|| Highlighter {
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme_set: ThemeSet::load_defaults(),
    })
}

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

fn convert_color(style: Style) -> Color32 {
    let c = style.foreground;
    Color32::from_rgba_premultiplied(c.r, c.g, c.b, c.a)
}
