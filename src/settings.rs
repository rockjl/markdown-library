use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontChoice {
    SystemDefault,
    SegoeUI,
    Arial,
    Consolas,
}

impl Default for FontChoice {
    fn default() -> Self {
        FontChoice::SystemDefault
    }
}

impl FontChoice {
    pub fn display_name(self) -> &'static str {
        match self {
            FontChoice::SystemDefault => "System Default",
            FontChoice::SegoeUI => "Segoe UI",
            FontChoice::Arial => "Arial",
            FontChoice::Consolas => "Consolas",
        }
    }

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

    pub fn all() -> &'static [FontChoice] {
        &[
            FontChoice::SystemDefault,
            FontChoice::SegoeUI,
            FontChoice::Arial,
            FontChoice::Consolas,
        ]
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    EditorOnly,
    Split,
    PreviewOnly,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Split
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default = "default_font_size")]
    pub editor_font_size: f32,
    #[serde(default)]
    pub font_choice: FontChoice,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_true")]
    pub show_toolbar: bool,
    #[serde(default)]
    pub toolbar_collapsed: bool,
    #[serde(default = "default_true")]
    pub show_sidebar: bool,
    #[serde(default)]
    pub view_mode: ViewMode,
    #[serde(default = "default_true")]
    pub syntax_highlight: bool,
    #[serde(default)]
    pub show_toc: bool,
    #[serde(default = "default_true")]
    pub sync_scroll: bool,
    #[serde(default = "default_sidebar_batch")]
    pub sidebar_batch: usize,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
}

fn default_font_size() -> f32 {
    13.0
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            editor_font_size: 13.0,
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
        }
    }
}

fn default_sidebar_batch() -> usize {
    80
}

fn default_sidebar_width() -> f32 {
    280.0
}
