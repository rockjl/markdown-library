//! Application settings (UI theme, font, layout, etc.), serialized as JSON.

use serde::{Deserialize, Serialize};

/// Font family selection for the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontChoice {
    /// Use egui's built-in default fonts
    SystemDefault,
    /// Windows Segoe UI (fallback chain on other OS)
    SegoeUI,
    /// Arial (fallback chain on other OS)
    Arial,
    /// Consolas / Menlo / Liberation Mono
    Consolas,
}

impl Default for FontChoice {
    fn default() -> Self {
        FontChoice::SystemDefault
    }
}

impl FontChoice {
    /// Human-readable display name for the font choice.
    pub fn display_name(self) -> &'static str {
        match self {
            FontChoice::SystemDefault => "System Default",
            FontChoice::SegoeUI => "Segoe UI",
            FontChoice::Arial => "Arial",
            FontChoice::Consolas => "Consolas",
        }
    }

    /// Candidate font file paths to try, ordered by preference.
    pub fn font_candidates(self) -> &'static [&'static str] {
        match self {
            FontChoice::SystemDefault => &[],
            FontChoice::SegoeUI => &[
                r"C:\Windows\Fonts\SegoeUI.ttf",
                r"C:\Windows\Fonts\segoeui.ttf",
                "/System/Library/Fonts/Helvetica.ttc",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            ],
            FontChoice::Arial => &[
                r"C:\Windows\Fonts\arial.ttf",
                r"C:\Windows\Fonts\Arial.ttf",
                "/System/Library/Fonts/Arial.ttf",
                "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf",
            ],
            FontChoice::Consolas => &[
                r"C:\Windows\Fonts\consola.ttf",
                r"C:\Windows\Fonts\Consolas.ttf",
                "/System/Library/Fonts/Menlo.ttc",
                "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            ],
        }
    }

    /// List all available font choices.
    pub fn all() -> &'static [FontChoice] {
        &[
            FontChoice::SystemDefault,
            FontChoice::SegoeUI,
            FontChoice::Arial,
            FontChoice::Consolas,
        ]
    }
}

/// UI colour theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::Dark
    }
}

/// Editor view mode (editor only, split, or preview only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    /// Only the source editor is shown
    EditorOnly,
    /// Editor and preview side by side
    Split,
    /// Only the rendered preview is shown
    PreviewOnly,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Split
    }
}

/// Persisted application settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    /// Colour theme (dark / light)
    #[serde(default)]
    pub theme: ThemeMode,
    /// Editor monospace font size in points
    #[serde(default = "default_font_size")]
    pub editor_font_size: f32,
    /// Preview proportional font size in points
    #[serde(default = "default_preview_font_size")]
    pub preview_font_size: f32,
    /// Selected font family
    #[serde(default)]
    pub font_choice: FontChoice,
    /// Whether line numbers are shown in the editor gutter
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    /// Whether word wrap is enabled in the editor
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    /// Whether auto-save is enabled
    #[serde(default = "default_true")]
    pub auto_save: bool,
    /// Whether the formatting toolbar is visible
    #[serde(default = "default_true")]
    pub show_toolbar: bool,
    /// Whether the toolbar is collapsed to a compact row
    #[serde(default)]
    pub toolbar_collapsed: bool,
    /// Whether the sidebar is visible
    #[serde(default = "default_true")]
    pub show_sidebar: bool,
    /// Current view mode
    #[serde(default)]
    pub view_mode: ViewMode,
    /// Whether syntax highlighting is enabled
    #[serde(default = "default_true")]
    pub syntax_highlight: bool,
    /// Whether the Table of Contents panel is shown
    #[serde(default)]
    pub show_toc: bool,
    /// Whether editor and preview scroll in sync
    #[serde(default = "default_true")]
    pub sync_scroll: bool,
    /// Number of notes to load per batch in the sidebar
    #[serde(default = "default_sidebar_batch")]
    pub sidebar_batch: usize,
    /// Sidebar width in egui points
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    /// Number of recently viewed notes to show in the sidebar history
    #[serde(default = "default_recent_count")]
    pub recent_count: usize,
}

fn default_font_size() -> f32 { 13.0 }
fn default_preview_font_size() -> f32 { 14.0 }
fn default_true() -> bool { true }
fn default_sidebar_batch() -> usize { 80 }
fn default_sidebar_width() -> f32 { 280.0 }
fn default_recent_count() -> usize { 10 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            editor_font_size: 13.0,
            preview_font_size: 14.0,
            font_choice: FontChoice::default(),
            show_line_numbers: true,
            word_wrap: true,
            auto_save: true,
            show_toolbar: true,
            show_sidebar: true,
            view_mode: ViewMode::Split,
            syntax_highlight: true,
            show_toc: false,
            sync_scroll: true,
            sidebar_batch: 80,
            sidebar_width: 280.0,
            toolbar_collapsed: false,
            recent_count: 10,
        }
    }
}
