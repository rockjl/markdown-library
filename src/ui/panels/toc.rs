use crate::ui::app::MarkdownApp;
use egui::{RichText, ScrollArea, Ui};
use crate::ui::constants::EDITOR_ID;

impl MarkdownApp {
    pub fn draw_toc(&mut self, ui: &mut Ui) {
        let Some(sel) = self.selected else { return; };
        let c = self.colors();
        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.label(RichText::new("📑  TOC").color(c.text_dim).size(12.0));
            });
        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        let content = self.notes[sel].content.clone();
        let headings = crate::toc::extract(&content);

        ScrollArea::vertical()
            .id_salt("toc_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);
                if headings.is_empty() {
                    ui.label(
                        RichText::new("  No headings")
                            .color(c.text_dim)
                            .size(11.0)
                            .italics(),
                    );
                    return;
                }
                let mut jump_to: Option<usize> = None;
                for h in &headings {
                    let indent = (h.level.saturating_sub(1) as f32) * 12.0;
                    let resp = ui.horizontal(|ui| {
                        ui.add_space(8.0 + indent);
                        let color = if h.level == 1 { c.text_strong } else { c.text_normal };
                        ui.add(egui::Label::new(
                            RichText::new(&h.text).size(12.0).color(color),
                        ).sense(egui::Sense::click()))
                    });
                    if resp.inner.clicked() {
                        jump_to = Some(h.char_offset);
                    }
                }
                if let Some(offset) = jump_to {
                    let editor_id = egui::Id::new(EDITOR_ID);
                    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                        let range = egui::text::CCursorRange::two(
                            egui::text::CCursor::new(offset),
                            egui::text::CCursor::new(offset),
                        );
                        state.cursor.set_char_range(Some(range));
                        state.store(ui.ctx(), editor_id);
                    }
                    ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                }
            });
    }
}
