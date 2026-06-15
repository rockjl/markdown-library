// Copyright (c) 2025 markdown-library

use crate::ui::app::MarkdownApp;
use crate::settings::ViewMode;
use crate::wikilinks;
use egui::{FontFamily, FontId, RichText, ScrollArea, TextStyle, Ui};
use egui_commonmark::CommonMarkViewer;

impl MarkdownApp {
    pub fn draw_preview(&mut self, ui: &mut Ui) {
        let Some(sel) = self.selected else {
            return;
        };

        let c = self.colors();
        let raw = &self.notes[sel].content;
        let content = wikilinks::render_for_preview(raw);

        egui::Frame::none().fill(c.preview_bg).show(ui, |ui| {
            egui::Frame::none()
                .fill(c.header_bg)
                .inner_margin(egui::Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("👁  Preview")
                                .color(c.text_dim)
                                .size(12.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mode = self.settings.view_mode;
                            if ui
                                .selectable_label(mode == ViewMode::PreviewOnly, "👁")
                                .on_hover_text("Preview only · Ctrl+. · Ctrl+, edit")
                                .clicked()
                            {
                                self.settings.view_mode = ViewMode::PreviewOnly;
                                self.save_settings();
                            }
                            if ui
                                .selectable_label(mode == ViewMode::Split, "⫼")
                                .on_hover_text("Split view · Ctrl+. preview · Ctrl+, edit")
                                .clicked()
                            {
                                self.settings.view_mode = ViewMode::Split;
                                self.save_settings();
                            }
                            if ui
                                .selectable_label(mode == ViewMode::EditorOnly, "📝")
                                .on_hover_text("Editor only · Ctrl+, · Ctrl+. preview")
                                .clicked()
                            {
                                self.settings.view_mode = ViewMode::EditorOnly;
                                self.save_settings();
                            }
                        });
                    });
                });

            ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

            ScrollArea::vertical()
                .id_salt("preview_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(ui.available_width());
                    ui.style_mut().text_styles.insert(
                        TextStyle::Body,
                        FontId::new(self.settings.preview_font_size, FontFamily::Proportional),
                    );
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(20, 16))
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width());
                            CommonMarkViewer::new()
                                .max_image_width(Some(600))
                                .show(ui, &mut self.cache, &content);
                        });
                });
        });
    }
}
