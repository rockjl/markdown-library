use crate::app::MarkdownApp;
use crate::wikilinks;
use egui::{RichText, ScrollArea, Ui};
use crate::ui::types::NavTarget;

impl MarkdownApp {
    pub fn draw_backlinks_panel(&mut self, ui: &mut Ui) {
        let Some(sel) = self.selected else { return; };
        let c = self.colors();
        let current_title = self.notes[sel].title.clone();

        let backlinks_index = wikilinks::build_backlink_index(&self.notes);
        let target_idx = self.current_backlinks_target.unwrap_or(sel);
        let backlinks = backlinks_index.get(&target_idx).cloned().unwrap_or_default();
        let outgoing = wikilinks::extract(&self.notes[target_idx].content);


        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔗  Links").color(c.text_dim).size(12.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").on_hover_text("Close").clicked() {
                            self.show_backlinks = false;
                        }
                    });
                });
            });
        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        let mut nav_target: Option<NavTarget> = None;

        ScrollArea::vertical()
            .id_salt("backlinks_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);

                ui.label(
                    RichText::new(format!("  📤 Outgoing ({})", outgoing.len()))
                        .color(c.text_dim)
                        .size(11.0)
                        .strong(),
                );
                if outgoing.is_empty() {
                    ui.label(
                        RichText::new("    (なし)")
                            .color(c.text_dim)
                            .size(11.0)
                            .italics(),
                    );
                } else {
                    for link in &outgoing {
                        let display = link.alias.as_deref().unwrap_or(&link.target);
                        let exists = !wikilinks::resolve(&self.notes, &link.target).is_empty();
                        let color = if exists { c.accent } else { c.text_dim };
                        let prefix = if exists { "  → " } else { "  ✚ " };
                        let resp = ui.add(
                            egui::Label::new(
                                RichText::new(format!("{}{}", prefix, display))
                                    .color(color)
                                    .size(11.0),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            nav_target = Some(NavTarget::Wiki(link.target.clone()));
                        }
                        if !exists {
                            resp.on_hover_text("Click to create a new note");
                        }
                    }
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("  📥 Backlinks ({})", backlinks.len()))
                        .color(c.text_dim)
                        .size(11.0)
                        .strong(),
                );
                if backlinks.is_empty() {
                    ui.label(
                        RichText::new("    (なし)")
                            .color(c.text_dim)
                            .size(11.0)
                            .italics(),
                    );
                } else {
                    for (src_idx, link) in &backlinks {
                        let src_title = &self.notes[*src_idx].title;
                        let label = if let Some(alias) = &link.alias {
                            format!("  ← {} ({})", src_title, alias)
                        } else {
                            format!("  ← {}", src_title)
                        };
                        let resp = ui.add(
                            egui::Label::new(
                                RichText::new(label).color(c.text_normal).size(11.0),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            nav_target = Some(NavTarget::Index(*src_idx));
                        }
                    }
                }
                let _ = current_title;
            });

        match nav_target {
            Some(NavTarget::Wiki(target)) => self.navigate_to_wikilink(&target),
            Some(NavTarget::Index(idx)) => self.select_note_by_index(idx),
            None => {}
        }
        self.current_backlinks_target = None;
    }
}
