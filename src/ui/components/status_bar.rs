use egui::RichText;

use crate::ui::app::MarkdownApp;

impl MarkdownApp {
    pub fn draw_status_bar(&mut self, ctx: &egui::Context) {
        let c = self.colors();
        let dirty = self.notes_dirty;
        let auto_save = self.settings.auto_save;
        if let Some(at) = self.status_override_at {
            if at.elapsed().as_secs() >= 6 {
                self.status_override = None;
                self.status_override_at = None;
            }
        }
        let status_override = self.status_override.clone();
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none().fill(c.menu_bg).inner_margin(egui::Margin::symmetric(12, 6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut status_parts = Vec::new();
                    if self.voice_engine.as_ref().map_or(false, |v| v.is_running()) {
                        if !self.current_transcript.is_empty() {
                            status_parts.push(format!("🎤 Rec · \"{}\"", self.current_transcript));
                        } else {
                            status_parts.push("🎤 Recording...".to_string());
                        }
                    }
                    if let Some(s) = status_override.as_deref() {
                        status_parts.push(s.to_string());
                    } else if dirty {
                        if auto_save { status_parts.push("● Auto-saving…".to_string()); }
                        else { status_parts.push("● Unsaved changes".to_string()); }
                    } else {
                        status_parts.push("✓ Saved".to_string());
                    }
                    let status = status_parts.join(" · ");
                    ui.label(RichText::new(status).color(c.text_dim).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} files", self.notes.len()))
                                .color(c.text_dim)
                                .size(11.0),
                        );
                    });
                });
            });
    }
}
