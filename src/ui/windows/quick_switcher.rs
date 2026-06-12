use crate::ui::app::MarkdownApp;
use crate::note::Note;
use crate::wikilinks;
use egui::{Align2, Color32, FontFamily, FontId, Margin, RichText, Sense, TextEdit};

impl MarkdownApp {
    pub(crate) fn draw_quick_switcher(&mut self, ctx: &egui::Context) {
        if !self.quick_switcher.visible {
            return;
        }
        let c = self.colors();
        let mut close = false;
        let mut navigate_to: Option<usize> = None;

        let query = self.quick_switcher.query.clone();
        let mut candidates: Vec<(i32, usize)> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.trashed)
            .filter_map(|(i, n)| wikilinks::fuzzy_match(&n.title, &query).map(|s| (s, i)))
            .collect();
        candidates.sort_by_key(|(score, _)| *score);
        candidates.truncate(20);

        if self.quick_switcher.selected >= candidates.len() {
            self.quick_switcher.selected = 0;
        }

        let (key_up, key_down, key_enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
            )
        });
        if !candidates.is_empty() {
            if key_down {
                self.quick_switcher.selected =
                    (self.quick_switcher.selected + 1) % candidates.len();
            }
            if key_up {
                self.quick_switcher.selected = (self.quick_switcher.selected + candidates.len() - 1)
                    % candidates.len();
            }
            if key_enter {
                navigate_to = Some(candidates[self.quick_switcher.selected].1);
                close = true;
            }
        } else if key_enter && !query.is_empty() {
            let mut note = Note::new(query.clone(), format!("# {}\n\n", query));
            note.modified = true;
            self.notes.push(note);
            navigate_to = Some(self.notes.len() - 1);
            self.notes_dirty = true;
            close = true;
        }

        egui::Window::new("Quick Switcher")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_TOP, [0.0, 80.0])
            .default_width(480.0)
            .frame(
                egui::Frame::popup(&ctx.style())
                    .fill(c.toolbar_bg)
                    .rounding(8.0)
                    .inner_margin(Margin::same(10)),
            )
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔎").size(14.0));
                    let resp = ui.add(
                        TextEdit::singleline(&mut self.quick_switcher.query)
                            .hint_text("Search notes... (Ctrl+P)")
                            .desired_width(f32::INFINITY)
                            .font(FontId::new(14.0, FontFamily::Proportional))
                            .frame(egui::Frame::NONE),
                    );
                    if self.quick_switcher.focus_query {
                        resp.request_focus();
                        self.quick_switcher.focus_query = false;
                    }
                });
                ui.add_space(6.0);
                ui.add(egui::Separator::default().spacing(0.0).grow(0.0));
                ui.add_space(4.0);

                if candidates.is_empty() {
                    if query.is_empty() {
                        ui.label(
                            RichText::new("Please enter a note name")
                                .color(c.text_dim)
                                .italics(),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!(
                                "Press Enter to create a new note named \"{}\"",
                                query
                            ))
                            .color(c.accent),
                        );
                    }
                } else {
                    for (rank, (_, idx)) in candidates.iter().enumerate() {
                        let is_active = rank == self.quick_switcher.selected;
                        let note = &self.notes[*idx];
                        let bg = if is_active {
                            c.selected_item_bg
                        } else {
                            Color32::TRANSPARENT
                        };
                        let resp = egui::Frame::none()
                            .fill(bg)
                            .rounding(4.0)
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(if note.starred { "★ " } else { "  " })
                                            .color(Color32::from_rgb(255, 200, 60)),
                                    );
                                    ui.label(
                                        RichText::new(&note.title)
                                            .color(if is_active {
                                                Color32::WHITE
                                            } else {
                                                c.text_normal
                                            })
                                            .size(13.0),
                                    );
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if resp.clicked() {
                            navigate_to = Some(*idx);
                            close = true;
                        }
                        if resp.hovered() {
                            self.quick_switcher.selected = rank;
                        }
                    }
                }
                ui.add_space(4.0);
                ui.add(egui::Separator::default().spacing(0.0).grow(0.0));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("↑↓ Move / Enter = Select / Esc = Close")
                            .color(c.text_dim)
                            .size(10.0),
                    );
                });
            });

        if let Some(idx) = navigate_to {
            self.selected = Some(idx);
        }
        if close {
            self.quick_switcher.close();
        }
    }
}
