use egui::RichText;

use crate::ui::app::MarkdownApp;
use crate::theme;
use crate::settings::{ThemeMode, ViewMode};
use crate::editor_actions::EditorAction;

impl MarkdownApp {
    pub fn draw_menu_bar(&mut self, ctx: &egui::Context) {
        let c = self.colors();
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(c.menu_bg)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
                    ui.menu_button("File", |ui| {
                        if ui.button("New note  Ctrl+N").clicked() {
                            self.new_note();
                            ui.close_menu();
                        }
                        if ui.button("Open...  Ctrl+O").clicked() {
                            self.open_file();
                            ui.close_menu();
                        }
                        if ui.button("Import Q&A...").clicked() {
                            self.import_qa_file();
                            ui.close_menu();
                        }
                        if ui.button("Save to file...  Ctrl+S").clicked() {
                            self.save_current_to_file();
                            ui.close_menu();
                        }
                        if ui.button("Save all").clicked() {
                            self.save_notes();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export to HTML...").clicked() {
                            self.export_html();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("View", |ui| {
                        ui.checkbox(&mut self.settings.show_sidebar, "Sidebar");
                        ui.checkbox(&mut self.settings.show_toc, "Show TOC (Table of Contents)");
                        ui.checkbox(&mut self.show_backlinks, "Show Backlinks");
                        ui.separator();
                        ui.label(RichText::new("View mode").small().color(c.text_dim));
                        ui.radio_value(&mut self.settings.view_mode, ViewMode::EditorOnly, "📝 Editor only");
                        ui.radio_value(&mut self.settings.view_mode, ViewMode::Split, "⫼ Split view");
                        ui.radio_value(&mut self.settings.view_mode, ViewMode::PreviewOnly, "👁 Preview only");
                        ui.separator();
                        ui.checkbox(&mut self.settings.show_line_numbers, "Line numbers");
                        ui.checkbox(&mut self.settings.word_wrap, "Word wrap");
                        ui.checkbox(&mut self.settings.sync_scroll, "Sync scroll");
                    });
                    ui.menu_button("Edit", |ui| {
                        if ui.button("Quick switcher  Ctrl+P").clicked() {
                            self.quick_switcher.open();
                            ui.close_menu();
                        }
                        if ui.button("Find...  Ctrl+F").clicked() {
                            self.find.open_find();
                            ui.close_menu();
                        }
                        if ui.button("Replace...  Ctrl+H").clicked() {
                            self.find.open_replace();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("📋 Paste image from clipboard").clicked() {
                            self.try_paste_image(true);
                            ui.close_menu();
                        }
                        if ui.button("Insert Wiki link  [[]]").clicked() {
                            self.pending_action = Some(EditorAction::Insert("[[]]"));
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Theme", |ui| {
                        if ui.radio_value(&mut self.settings.theme, ThemeMode::Dark, "🌙 Dark").clicked() {
                            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
                            self.save_settings();
                            ui.close_menu();
                        }
                        if ui.radio_value(&mut self.settings.theme, ThemeMode::Light, "☀ Light").clicked() {
                            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
                            self.save_settings();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Tools", |ui| {
                        if ui.button("⚙ Settings...").clicked() {
                            self.show_settings = true;
                            ui.close_menu();
                        }
                    });
                });
            });
    }
}
