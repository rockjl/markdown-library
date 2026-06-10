use crate::settings::ThemeMode;
use egui::{Color32, FontFamily, FontId, Style, TextStyle, Visuals};

#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub sidebar_bg: Color32,
    pub editor_bg: Color32,
    pub preview_bg: Color32,
    pub toolbar_bg: Color32,
    pub header_bg: Color32,
    pub selected_item_bg: Color32,
    pub hover_item_bg: Color32,
    pub accent: Color32,
    pub text_dim: Color32,
    pub text_normal: Color32,
    pub text_strong: Color32,
    pub line_number: Color32,
    pub button_bg: Color32,
    pub menu_bg: Color32,
}

pub const DARK: ThemeColors = ThemeColors {
    sidebar_bg: Color32::from_rgb(24, 26, 30),
    editor_bg: Color32::from_rgb(30, 32, 38),
    preview_bg: Color32::from_rgb(34, 36, 42),
    toolbar_bg: Color32::from_rgb(32, 34, 40),
    header_bg: Color32::from_rgb(26, 28, 34),
    selected_item_bg: Color32::from_rgb(40, 160, 120),
    hover_item_bg: Color32::from_rgb(42, 45, 54),
    accent: Color32::from_rgb(45, 180, 140),
    text_dim: Color32::from_rgb(120, 125, 140),
    text_normal: Color32::from_rgb(200, 205, 215),
    text_strong: Color32::WHITE,
    line_number: Color32::from_rgb(80, 85, 100),
    button_bg: Color32::from_rgb(40, 42, 50),
    menu_bg: Color32::from_rgb(20, 22, 26),
};

pub const LIGHT: ThemeColors = ThemeColors {
    sidebar_bg: Color32::from_rgb(245, 246, 248),
    editor_bg: Color32::from_rgb(255, 255, 255),
    preview_bg: Color32::from_rgb(252, 252, 253),
    toolbar_bg: Color32::from_rgb(240, 242, 245),
    header_bg: Color32::from_rgb(238, 240, 243),
    selected_item_bg: Color32::from_rgb(40, 160, 120),
    hover_item_bg: Color32::from_rgb(225, 228, 232),
    accent: Color32::from_rgb(35, 150, 110),
    text_dim: Color32::from_rgb(120, 125, 140),
    text_normal: Color32::from_rgb(40, 44, 52),
    text_strong: Color32::from_rgb(20, 22, 28),
    line_number: Color32::from_rgb(170, 175, 185),
    button_bg: Color32::from_rgb(230, 232, 236),
    menu_bg: Color32::from_rgb(235, 237, 240),
};

pub fn colors(mode: ThemeMode) -> ThemeColors {
    match mode {
        ThemeMode::Dark => DARK,
        ThemeMode::Light => LIGHT,
    }
}

pub fn apply(ctx: &egui::Context, mode: ThemeMode, font_size: f32) {
    let c = colors(mode);
    let mut style = Style::default();

    style.text_styles = [
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(font_size, FontFamily::Monospace)),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(10.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.menu_margin = egui::Margin::symmetric(6, 4);
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.interact_size.y = 24.0;

    let mut visuals = match mode {
        ThemeMode::Dark => Visuals::dark(),
        ThemeMode::Light => Visuals::light(),
    };

    visuals.window_fill = c.sidebar_bg;
    visuals.panel_fill = c.sidebar_bg;
    visuals.faint_bg_color = c.toolbar_bg;
    visuals.extreme_bg_color = c.editor_bg;
    visuals.selection.bg_fill = c.accent;
    visuals.hyperlink_color = c.accent;

    visuals.widgets.noninteractive.bg_fill = c.toolbar_bg;
    visuals.widgets.inactive.bg_fill = c.button_bg;
    visuals.widgets.hovered.bg_fill = c.hover_item_bg;
    visuals.widgets.active.bg_fill = c.accent;

    visuals.widgets.noninteractive.fg_stroke.color = c.text_dim;
    visuals.widgets.inactive.fg_stroke.color = c.text_normal;
    visuals.widgets.hovered.fg_stroke.color = c.text_strong;
    visuals.widgets.active.fg_stroke.color = c.text_strong;

    let border = if mode == ThemeMode::Dark {
        Color32::from_rgb(50, 53, 62)
    } else {
        Color32::from_rgb(215, 220, 226)
    };
    visuals.widgets.noninteractive.bg_stroke.color = border;
    visuals.widgets.noninteractive.bg_stroke.width = 1.0;

    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);

    style.visuals = visuals;
    ctx.set_style(style);
}
