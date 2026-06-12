use egui::{RichText, TextEdit};

use crate::app::MarkdownApp;
use crate::find_replace;
use super::super::constants::EDITOR_ID;

impl MarkdownApp {
    pub fn jump_to_match(&self, ctx: &egui::Context, matches: &[(usize, usize)]) {
        if matches.is_empty() {
            return;
        }
        let Some(sel) = self.selected else { return; };
        let (b_start, b_end) = matches[self.find.current_match.min(matches.len() - 1)];
        let content = &self.notes[sel].content;
        let char_start = content[..b_start].chars().count();
        let char_end = content[..b_end].chars().count();
        let editor_id = egui::Id::new(EDITOR_ID);
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) {
            let new_range = egui::text::CCursorRange::two(
                egui::text::CCursor::new(char_start),
                egui::text::CCursor::new(char_end),
            );
            state.cursor.set_char_range(Some(new_range));
            state.store(ctx, editor_id);
        }
    }

    pub fn replace_current_match(&mut self) {
        let Some(sel) = self.selected else { return; };
        let matches = find_replace::find_all(
            &self.notes[sel].content,
            &self.find.query,
            self.find.case_sensitive,
        );
        if matches.is_empty() {
            return;
        }
        let idx = self.find.current_match.min(matches.len() - 1);
        let (b_start, b_end) = matches[idx];
        let content = &self.notes[sel].content;
        let new_content = format!(
            "{}{}{}",
            &content[..b_start],
            self.find.replace_with,
            &content[b_end..]
        );
        let note = &mut self.notes[sel];
        note.content = new_content;
        note.modified = true;
        note.touch();
        self.notes_dirty = true;
    }

    pub fn replace_all(&mut self) {
        let Some(sel) = self.selected else { return; };
        let (new_content, count) = find_replace::replace_all(
            &self.notes[sel].content,
            &self.find.query,
            &self.find.replace_with,
            self.find.case_sensitive,
        );
        if count > 0 {
            let note = &mut self.notes[sel];
            note.content = new_content;
            note.modified = true;
            note.touch();
            self.notes_dirty = true;
        }
    }

    pub fn draw_find_bar(&mut self, ctx: &egui::Context) {
        if !self.find.visible {
            return;
        }
        let Some(sel) = self.selected else { return; };
        let c = self.colors();
        egui::TopBottomPanel::top("find_bar")
            .frame(
                egui::Frame::none()
                    .fill(c.toolbar_bg)
                    .inner_margin(egui::Margin::symmetric(8, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").color(c.text_dim));
                    let query_resp = ui.add(
                        TextEdit::singleline(&mut self.find.query)
                            .hint_text("Search...")
                            .desired_width(200.0),
                    );
                    if self.find.focus_query {
                        query_resp.request_focus();
                        self.find.focus_query = false;
                    }

                    let matches = find_replace::find_all(
                        &self.notes[sel].content,
                        &self.find.query,
                        self.find.case_sensitive,
                    );
                    let total = matches.len();
                    let current_display = if total == 0 {
                        "0/0".to_string()
                    } else {
                        let cur = self.find.current_match.min(total.saturating_sub(1)) + 1;
                        format!("{}/{}", cur, total)
                    };
                    ui.label(RichText::new(current_display).color(c.text_dim).size(11.0));

                    if ui.button("▲").on_hover_text("Previous match (Shift+Enter)").clicked() {
                        if total > 0 {
                            self.find.current_match = (self.find.current_match + total - 1) % total;
                            self.jump_to_match(ctx, &matches);
                        }
                    }
                    if ui.button("▼").on_hover_text("Next match (Enter)").clicked() {
                        if total > 0 {
                            self.find.current_match = (self.find.current_match + 1) % total;
                            self.jump_to_match(ctx, &matches);
                        }
                    }
                    ui.checkbox(&mut self.find.case_sensitive, "Aa")
                        .on_hover_text("大文字小文字を区別");

                    if ui.button("✖").on_hover_text("閉じる (Esc)").clicked() {
                        self.find.close();
                    }
                });

                if self.find.show_replace {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("↳").color(c.text_dim));
                        ui.add(
                            TextEdit::singleline(&mut self.find.replace_with)
                                .hint_text("Replace with...")
                                .desired_width(200.0),
                        );
                        if ui.button("Replace").clicked() {
                            self.replace_current_match();
                        }
                        if ui.button("Replace All").clicked() {
                            self.replace_all();
                        }
                    });
                }
            });

        if self.find.visible {
            let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            let shift = ctx.input(|i| i.modifiers.shift);
            if enter {
                let matches = find_replace::find_all(
                    &self.notes[sel].content,
                    &self.find.query,
                    self.find.case_sensitive,
                );
                let total = matches.len();
                if total > 0 {
                    if shift {
                        self.find.current_match = (self.find.current_match + total - 1) % total;
                    } else {
                        self.find.current_match = (self.find.current_match + 1) % total;
                    }
                    self.jump_to_match(ctx, &matches);
                }
            }
        }
    }
}
