use crate::app::load_user_font;
use crate::app::MarkdownApp;
use crate::settings::{FontChoice, ThemeMode};
use crate::storage;
use crate::theme;
use egui::{Align2, RichText};

impl MarkdownApp {
    pub(crate) fn draw_settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut open = self.show_settings;
        let mut settings_changed = false;
        let mut font_changed = false;
        egui::Window::new("⚙ Settings")
            .open(&mut open)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Appearance").strong().size(14.0));
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        if ui
                            .selectable_label(self.settings.theme == ThemeMode::Dark, "🌙 Dark")
                            .clicked()
                        {
                            self.settings.theme = ThemeMode::Dark;
                            settings_changed = true;
                        }
                        if ui
                            .selectable_label(self.settings.theme == ThemeMode::Light, "☀ Light")
                            .clicked()
                        {
                            self.settings.theme = ThemeMode::Light;
                            settings_changed = true;
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(RichText::new("Editor").strong().size(14.0));

                    ui.horizontal(|ui| {
                        ui.label("Font:");
                        let current = self.settings.font_choice;
                        egui::ComboBox::from_id_salt("font_choice")
                            .selected_text(current.display_name())
                            .show_ui(ui, |ui| {
                                for &choice in FontChoice::all() {
                                    if ui
                                        .selectable_label(current == choice, choice.display_name())
                                        .clicked()
                                    {
                                        self.settings.font_choice = choice;
                                        font_changed = true;
                                        settings_changed = true;
                                    }
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Font size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.settings.editor_font_size)
                                    .range(8.0..=32.0)
                                    .speed(0.5)
                                    .suffix(" px"),
                            )
                            .changed()
                        {
                            settings_changed = true;
                        }
                    });
                    if ui
                        .checkbox(&mut self.settings.show_line_numbers, "Show line numbers")
                        .changed()
                    {
                        settings_changed = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.word_wrap, "Word wrap")
                        .changed()
                    {
                        settings_changed = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.auto_save, "Auto save")
                        .changed()
                    {
                        settings_changed = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.syntax_highlight, "Syntax highlight")
                        .changed()
                    {
                        settings_changed = true;
                    }
                });

                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Storage").strong().size(14.0));
                    ui.horizontal(|ui| {
                        ui.label("Data directory:");
                        ui.monospace(storage::data_dir().display().to_string());
                    });

                    ui.horizontal(|ui| {
                        if ui.button("📂 Open data folder").clicked() {
                            let path = storage::data_dir();
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("explorer").arg(&path).spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open").arg(&path).spawn();
                            }
                            #[cfg(all(unix, not(target_os = "macos")))]
                            {
                                let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                            }
                        }


                    });
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("Restore Defaults")
                        .on_hover_text("Reset all settings to default values")
                        .clicked()
                    {
                        self.confirm_restore_defaults = true;
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new("Advanced").strong().size(14.0));
                ui.horizontal(|ui| {
                    ui.label("Sidebar batch size:");
                    let mut b = self.sidebar_batch as i32;
                    if ui.add(egui::DragValue::new(&mut b).range(8..=500)).changed() {
                        self.sidebar_batch = b.max(8) as usize;
                        settings_changed = true;
                    }
                });
                if ui
                    .checkbox(&mut self.settings.show_toolbar, "Show toolbar")
                    .on_hover_text("Toggle toolbar visibility")
                    .changed()
                {
                    settings_changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Sidebar width:");
                    let mut w = self.settings.sidebar_width;
                    if ui.add(egui::DragValue::new(&mut w).range(160.0..=800.0)).changed() {
                        self.settings.sidebar_width = w.max(160.0);
                        settings_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Recent notes count:");
                    let mut n = self.settings.recent_count as i32;
                    if ui.add(egui::DragValue::new(&mut n).range(1..=100)).changed() {
                        self.settings.recent_count = n.max(1) as usize;
                        settings_changed = true;
                        self.note_history.truncate(self.settings.recent_count);
                    }
                });
            });
        self.show_settings = open;
        if font_changed {
            load_user_font(ctx, self.settings.font_choice);
        }
        if settings_changed {
            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
            self.save_settings();
        }

        if self.confirm_restore_defaults {
            egui::Window::new("Confirm Restore Defaults")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("This will reset all settings to defaults. Continue?");
                    ui.horizontal(|ui| {
                        if ui.button("Yes, restore").clicked() {
                            self.settings = crate::settings::Settings::default();
                            self.toolbar_collapsed = self.settings.toolbar_collapsed;
                            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
                            self.save_settings();
                            self.confirm_restore_defaults = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_restore_defaults = false;
                        }
                    });
                });
        }
    }
}
