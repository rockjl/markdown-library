use crate::ui::app::MarkdownApp;
use crate::voice::VoiceEngine;
use egui::{Color32, FontFamily, FontId, RichText, ScrollArea, TextEdit, Ui};
use std::time::SystemTime;

impl MarkdownApp {
    pub fn draw_sidebar(&mut self, ui: &mut Ui) {
        let c = self.colors();

        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let btn_size = egui::vec2(32.0, 32.0);

                    let resp = ui.add(
                        egui::Button::new(RichText::new("+").size(16.0).color(c.accent))
                            .min_size(btn_size)
                    );
                    if resp.on_hover_text("New note (Ctrl+N)").clicked() {
                        self.new_note();
                    }

                    let is_recording = self.voice_engine.as_ref().map_or(false, |v| v.is_running());
                    let voice_label = if is_recording { "🔴" } else { "🎤" };
                    let voice_tip = if is_recording { "Stop recording" } else { "Voice input (F12)" };
                    let resp = ui.add(
                        egui::Button::new(RichText::new(voice_label).size(16.0))
                            .min_size(btn_size)
                    );
                    if resp.on_hover_text(voice_tip).clicked() {
                        if is_recording {
                            self.stop_and_search();
                        } else {
                            self.voice_engine = Some(VoiceEngine::start(
                                "c44dd50e",
                                "971738b63e6eb7bfd94a8246648ca421",
                            ));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (mode_label, mode_tip, fill_color, text_color) = if self.library_mode {
                            (
                                "📂",
                                "Switch to Interview mode",
                                Color32::from_rgb(60, 45, 20),
                                Color32::from_rgb(230, 180, 60),
                            )
                        } else {
                            (
                                "🎤",
                                "Switch to Library mode",
                                Color32::from_rgb(20, 60, 45),
                                c.accent,
                            )
                        };
                        let resp = ui.add(
                            egui::Button::new(RichText::new(mode_label).size(16.0).color(text_color))
                                .min_size(btn_size)
                                .rounding(16.0)
                                .fill(fill_color)
                        );
                        if resp.on_hover_text(mode_tip).clicked() {
                            self.library_mode = !self.library_mode;
                            self.selected = None;
                        }
                    });
                });
            });

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                let search_resp = ui.add(
                    TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍  Search...")
                        .desired_width(f32::INFINITY)
                        .font(FontId::new(13.0, FontFamily::Proportional)),
                )
                .on_hover_text("Ctrl+/ to focus · Enter to search");
                if self.focus_sidebar_search {
                    search_resp.request_focus();
                    self.focus_sidebar_search = false;
                }
                if !self.voice_search_results.is_empty() && !self.search_query.is_empty() {
                    self.voice_search_results.clear();
                }

                if search_resp.lost_focus() && ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                    let q = self.search_query.trim().to_string();
                    if !q.is_empty() {
                        self.search_history.retain(|x| x != &q);
                        self.search_history.insert(0, q.clone());
                        if self.search_history.len() > self.max_history {
                            self.search_history.truncate(self.max_history);
                        }
                        crate::storage::save_search_history(&self.search_history);
                    }
                }

                if !self.search_history.is_empty() {
                    egui::ComboBox::from_id_salt("search_history_cb")
                        .selected_text(self.search_history[0].clone())
                        .show_ui(ui, |ui| {
                            for (i, item) in self.search_history.iter().enumerate() {
                                if ui.selectable_label(false, item).clicked() {
                                    self.search_query = item.clone();
                                }
                                if i >= 20 { break; }
                            }
                        });
                }
            });

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        if self.library_mode {
            ui.horizontal(|ui| {
                let tag_label = match &self.selected_tag {
                    Some(t) => format!("# {}", t),
                    None => "🏷 All Tags".to_string(),
                };
                let mut tag_btn = egui::Button::new(RichText::new(tag_label).size(12.0));
                if self.selected_tag.is_some() {
                    tag_btn = tag_btn.fill(Color32::from_rgb(160, 60, 60));
                }
                let tag_resp = ui.add(tag_btn);
                if tag_resp.clicked() {
                    if self.selected_tag.is_some() {
                        self.selected_tag = None;
                        self.show_tag_popup = false;
                    } else {
                        self.show_tag_popup = !self.show_tag_popup;
                    }
                }
                let recent_label = if self.show_recent { "[Recent]" } else { "Recent" };
                if ui.add(egui::Button::new(RichText::new(recent_label).size(12.0))).clicked() {
                    self.show_recent = !self.show_recent;
                }
                if self.show_tag_popup {
                    let popup_pos = tag_resp.rect.left_bottom();
                    let popup_id = ui.next_auto_id();
                    let available_w = tag_resp.rect.width().max(200.0);
                    egui::Area::new(popup_id)
                        .fixed_pos(popup_pos)
                        .show(ui.ctx(), |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.set_min_width(available_w);
                                ui.set_max_width(available_w * 2.0);
                                ui.set_max_height(300.0);
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.set_min_height(0.0);
                                        for tag in self.all_tags() {
                                            let is_sel = self.selected_tag.as_deref() == Some(&tag);
                                            let tag_text = if is_sel {
                                                format!("* {}", tag)
                                            } else {
                                                tag.clone()
                                            };
                                            let btn = egui::Button::new(
                                                RichText::new(tag_text).size(11.0)
                                            );
                                            if ui.add(btn).clicked() {
                                                self.selected_tag = Some(tag);
                                                self.show_tag_popup = false;
                                            }
                                        }
                                    });
                                });
                            });
                        });
                }
            });
            ui.add(egui::Separator::default().spacing(2.0).grow(0.0));
        }

        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                if self.library_mode {
                    ui.horizontal(|ui| {
                        let all_btn = egui::SelectableLabel::new(
                        !self.show_trash,
                        RichText::new("📝 Notes").size(12.0).color(c.text_normal),
                    );
                    if ui.add(all_btn).clicked() {
                        self.show_trash = false;
                    }
                    let already_trash = self.show_trash;
                    let trash_btn = egui::SelectableLabel::new(
                        self.show_trash,
                        RichText::new(format!(
                            "🗑 Trash ({})",
                            self.notes.iter().filter(|n| n.trashed).count()
                        ))
                        .size(12.0)
                        .color(c.text_normal),
                    );
                    if ui.add(trash_btn).clicked() {
                        if already_trash {
                            self.confirm_empty_trash = true;
                        } else {
                            self.show_trash = true;
                        }
                    }
                });
                }
            });

        egui::Frame::none().fill(c.sidebar_bg).show(ui, |ui| {
            ui.add_space(4.0);
            ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let query = self.search_query.clone();
                let query_lc = query.to_lowercase();
                let show_trash = self.show_trash;

                if !self.voice_search_results.is_empty() {
                    let recording = self.voice_engine.as_ref().map_or(false, |v| v.is_running());
                    ui.label(RichText::new(
                        if recording { "🔴 Listening..." } else { "🔍 Voice Results" }
                    ).size(13.0).color(c.accent));
                    ui.add_space(4.0);
                    for hit in &self.voice_search_results {
                        let is_selected = self.selected.and_then(|s| self.notes.get(s)).map_or(false, |n| n.id == hit.note_id);
                        let note_idx = self.notes.iter().position(|n| n.id == hit.note_id);
                        let desired_height = 52.0;
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), desired_height),
                            egui::Sense::click(),
                        );
                        if response.clicked() {
                            if let Some(idx) = note_idx {
                                self.selected = Some(idx);
                                self.settings.view_mode = crate::settings::ViewMode::PreviewOnly;
                            }
                        }
                        let bg = if is_selected {
                            c.selected_item_bg
                        } else if response.hovered() {
                            c.hover_item_bg
                        } else {
                            c.sidebar_bg
                        };
                        if is_selected || response.hovered() {
                            let bg_rect = egui::Rect::from_min_max(
                                rect.min + egui::vec2(0.0, 2.0),
                                egui::pos2(rect.max.x, rect.max.y - 4.0),
                            );
                            ui.painter().rect_filled(bg_rect, 4.0, bg);
                        }
                        let title_pos = rect.min + egui::vec2(12.0, 8.0);
                        ui.painter().text(
                            title_pos,
                            egui::Align2::LEFT_TOP,
                            &hit.title,
                            egui::FontId::new(14.0, FontFamily::Proportional),
                            if is_selected { Color32::WHITE } else { c.text_normal },
                        );
                        let score_text = format!("score: {:.2}", hit.score);
                        let score_pos = rect.min + egui::vec2(12.0, 30.0);
                        ui.painter().text(
                            score_pos,
                            egui::Align2::LEFT_TOP,
                            score_text,
                            egui::FontId::new(11.0, FontFamily::Proportional),
                            c.text_dim,
                        );
                    }
                    return;
                }

                let filtered: Vec<usize> = if self.show_recent {
                    let hist: Vec<usize> = self.note_history.iter()
                        .filter_map(|(idx, _)| {
                            let n = &self.notes[*idx];
                            if n.trashed != show_trash { return None; }
                            if let Some(ref tag) = self.selected_tag {
                                if !n.tags.contains(tag) { return None; }
                            }
                            Some(*idx)
                        })
                        .collect();
                    if hist.is_empty() && self.library_mode && query_lc.is_empty() {
                        self.notes.iter()
                            .enumerate()
                            .filter(|(_, n)| n.trashed == show_trash)
                            .filter(|(_, n)| {
                                self.selected_tag.as_ref().map_or(true, |tag| n.tags.contains(tag))
                            })
                            .map(|(i, _)| i)
                            .collect()
                    } else {
                        hist
                    }
                } else if self.library_mode && query_lc.is_empty() {
                    self.notes.iter()
                        .enumerate()
                        .filter(|(_, n)| n.trashed == show_trash)
                        .filter(|(_, n)| {
                            self.selected_tag.as_ref().map_or(true, |tag| n.tags.contains(tag))
                        })
                        .map(|(i, _)| i)
                        .collect()
                } else if !query_lc.is_empty() {
                    if let Some(ref index) = self.search_index {
                        let hits = crate::search::matcher::search(index, &query, 0.0);
                        hits.into_iter()
                            .filter_map(|h| self.notes.iter().position(|n| n.id == h.note_id))
                            .filter(|&i| self.notes[i].trashed == show_trash)
                            .filter(|&i| {
                                self.selected_tag.as_ref().map_or(true, |tag| self.notes[i].tags.contains(tag))
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                let current_filter_key = format!("{}|{}|{}|{:?}|{}", query_lc, show_trash, self.library_mode, self.selected_tag, self.show_recent);
                if current_filter_key != self.sidebar_filter_key {
                    self.sidebar_filter_key = current_filter_key.clone();
                    self.sidebar_loaded_count = 0;
                    self.selected = None;
                }

                if self.sidebar_loaded_count == 0 {
                    self.sidebar_loaded_count = self.sidebar_batch.min(filtered.len());
                }

                self.sidebar_visible_indices = filtered.clone();
                let total_filtered = filtered.len();
                let to_render = self.sidebar_loaded_count.min(total_filtered);
                let indices: Vec<usize> = filtered.into_iter().take(to_render).collect();

                if !indices.is_empty() && self.selected.is_none() {
                    let idx = indices[0];
                    self.selected = Some(idx);
                    self.note_history.retain(|(i, _)| *i != idx);
                    let ts = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    self.note_history.insert(0, (idx, ts));
                    self.note_history.truncate(self.settings.recent_count);
                    self.settings.view_mode = if self.library_mode {
                        crate::settings::ViewMode::EditorOnly
                    } else {
                        crate::settings::ViewMode::PreviewOnly
                    };
                }

                let mut new_selected = self.selected;
                let mut trash_idx: Option<usize> = None;
                let mut restore_idx: Option<usize> = None;
                let mut delete_idx: Option<usize> = None;

                let mut last_item_rect: Option<egui::Rect> = None;

                for i in indices {
                    let is_selected = self.selected == Some(i);
                    let title = self.notes[i].display_title();
                    let tags = &self.notes[i].tags;
                    let has_tags = !tags.is_empty();
                    let recent_ts = if self.show_recent {
                        self.note_history.iter().find(|(idx, _)| *idx == i).map(|(_, ts)| *ts)
                    } else {
                        None
                    };
                    let has_ts = recent_ts.is_some();
                    let desired_height = match (has_tags, has_ts) {
                        (false, false) => 38.0,
                        (true, false) => 58.0,
                        (false, true) => 50.0,
                        (true, true) => 70.0,
                    };
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), desired_height),
                        egui::Sense::click(),
                    );

                    let clicked = response.clicked();
                    let hovered = response.hovered();

                    if clicked {
                        new_selected = Some(i);
                    }

                    let bg = if is_selected {
                        c.selected_item_bg
                    } else if hovered {
                        c.hover_item_bg
                    } else {
                        c.sidebar_bg
                    };
                    if is_selected || hovered {
                        let bg_rect = egui::Rect::from_min_max(
                            rect.min + egui::vec2(0.0, 2.0),
                            egui::pos2(rect.max.x, rect.max.y - 4.0),
                        );
                        ui.painter().rect_filled(bg_rect, 4.0, bg);
                    }

                    ui.painter().line_segment(
                        [egui::pos2(rect.min.x, rect.max.y - 1.0), egui::pos2(rect.max.x, rect.max.y - 1.0)],
                        egui::Stroke::new(1.0, c.text_dim.gamma_multiply(0.25)),
                    );

                    let title_color = if is_selected {
                        Color32::WHITE
                    } else {
                        c.text_normal
                    };
                    let title_font = egui::FontId::new(13.0, FontFamily::Proportional);
                    let max_title_w = rect.width() - 24.0;
                    let galley = ui.painter().layout_no_wrap(
                        title.clone(),
                        title_font.clone(),
                        title_color,
                    );
                    let display_title = if galley.size().x > max_title_w {
                        let mut lo: usize = 0;
                        let mut hi = title.len();
                        while lo < hi {
                            let mid = (lo + hi + 1) / 2;
                            let prefix: String = title.chars().take(mid).collect();
                            let test = format!("{}…", prefix);
                            let test_galley = ui.painter().layout_no_wrap(
                                test,
                                title_font.clone(),
                                title_color,
                            );
                            if test_galley.size().x <= max_title_w {
                                lo = mid;
                            } else {
                                hi = mid - 1;
                            }
                        }
                        let prefix: String = title.chars().take(lo).collect();
                        format!("{}…", prefix)
                    } else {
                        title.clone()
                    };
                    let title_galley = ui.painter().layout_no_wrap(
                        display_title.clone(),
                        title_font.clone(),
                        title_color,
                    );
                    let title_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 12.0, rect.min.y + 10.0),
                        title_galley.size(),
                    );
                    let title_resp = ui.allocate_rect(title_rect, egui::Sense::hover());
                    title_resp.on_hover_text(title.clone());
                    ui.painter().text(
                        egui::pos2(rect.min.x + 12.0, rect.min.y + 10.0),
                        egui::Align2::LEFT_TOP,
                        display_title,
                        title_font,
                        title_color,
                    );

                    if let Some(ts) = recent_ts {
                        let ts_text = self.format_recent_time(ts);
                        let ts_pos = rect.min + egui::vec2(12.0, 26.0);
                        ui.painter().text(
                            ts_pos,
                            egui::Align2::LEFT_TOP,
                            ts_text,
                            egui::FontId::new(10.0, FontFamily::Proportional),
                            c.text_dim,
                        );
                    }

                    let y_off: f32 = if has_ts { 40.0 } else { 28.0 };

                    if has_tags {
                        let chip_y = rect.min.y + y_off;
                        let max_tag_x = rect.right() - 12.0;
                        let mut tag_x = rect.min.x + 12.0;
                        let show_count = tags.len().min(3);
                        for (ti, tag) in tags.iter().enumerate().take(show_count) {
                            let truncated = if tag.chars().count() > 12 {
                                format!("{}…", tag.chars().take(12).collect::<String>())
                            } else {
                                tag.clone()
                            };
                            let img_text = format!(" {} ", truncated);
                            let galley = ui.painter().layout_no_wrap(
                                img_text.clone(),
                                egui::FontId::new(10.0, FontFamily::Proportional),
                                c.text_normal,
                            );
                            let chip_w = galley.size().x + 8.0;
                            if tag_x + chip_w > max_tag_x {
                                break;
                            }
                            let chip_rect = egui::Rect::from_min_size(
                                egui::pos2(tag_x - 2.0, chip_y - 1.0),
                                egui::vec2(chip_w, 16.0),
                            );
                            let chip_resp = ui.allocate_rect(chip_rect, egui::Sense::hover());
                            ui.painter().rect_filled(chip_rect, 4.0, c.toolbar_bg);
                            ui.painter().galley(
                                egui::pos2(tag_x, chip_y),
                                galley,
                                c.text_normal,
                            );
                            chip_resp.on_hover_text(tag);
                            tag_x += chip_w + 4.0;
                            if ti == show_count - 1 && tags.len() > 3 {
                                let remaining = tags.len() - 3;
                                let overflow_text = format!(" +{}", remaining);
                                let overflow_galley = ui.painter().layout_no_wrap(
                                    overflow_text.clone(),
                                    egui::FontId::new(10.0, FontFamily::Proportional),
                                    c.text_dim,
                                );
                                let overflow_w = overflow_galley.size().x + 8.0;
                                if tag_x + overflow_w <= max_tag_x {
                                    let overflow_rect = egui::Rect::from_min_size(
                                        egui::pos2(tag_x - 2.0, chip_y - 1.0),
                                        egui::vec2(overflow_w, 16.0),
                                    );
                                    let overflow_resp = ui.allocate_rect(overflow_rect, egui::Sense::hover());
                                    ui.painter().rect_filled(overflow_rect, 4.0, c.toolbar_bg);
                                    ui.painter().galley(
                                        egui::pos2(tag_x, chip_y),
                                        overflow_galley,
                                        c.text_dim,
                                    );
                                    let remaining_tags: String = tags[3..]
                                        .iter()
                                        .map(|t| t.as_str())
                                        .collect::<Vec<&str>>()
                                        .join(", ");
                                    overflow_resp.on_hover_text(remaining_tags);
                                }
                            }
                        }
                    }

                    response.context_menu(|ui| {
                        if !show_trash {
                            if ui.button("🔎 Show Backlinks").clicked() {
                                self.selected = Some(i);
                                self.current_backlinks_target = Some(i);
                                self.show_backlinks = true;
                                ui.close_menu();
                            }
                            if ui.button("🗑 Move to Trash").clicked() {
                                trash_idx = Some(i);
                                ui.close_menu();
                            }
                        } else {
                            if ui.button("↩ Restore").clicked() {
                                restore_idx = Some(i);
                                ui.close_menu();
                            }
                            if ui.button("❌ Delete permanently").clicked() {
                                delete_idx = Some(i);
                                ui.close_menu();
                            }
                        }
                    });

                    ui.add_space(2.0);
                    last_item_rect = Some(rect);
                }

                let selection_changed = self.selected != new_selected;
                self.selected = new_selected;
                if selection_changed {
                    if let Some(idx) = self.selected {
                        self.note_history.retain(|(i, _)| *i != idx);
                        let ts = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        self.note_history.insert(0, (idx, ts));
                        self.note_history.truncate(self.settings.recent_count);
                        if self.library_mode {
                            self.settings.view_mode = crate::settings::ViewMode::EditorOnly;
                        } else if self.notes[idx].path.is_some() {
                            self.settings.view_mode = crate::settings::ViewMode::PreviewOnly;
                        } else {
                            self.settings.view_mode = crate::settings::ViewMode::EditorOnly;
                        }
                    }
                }

                if let Some(i) = trash_idx {
                    self.move_to_trash(i);
                }
                if let Some(i) = restore_idx {
                    self.restore_from_trash(i);
                }
                if let Some(i) = delete_idx {
                    self.delete_permanently(i);
                }
                let mut auto_loaded = false;
                if let Some(r) = last_item_rect {
                    let clip_max_y = ui.clip_rect().max.y;
                    if r.max.y <= clip_max_y + 8.0 && to_render < total_filtered {
                        self.sidebar_loaded_count = (to_render + self.sidebar_batch).min(total_filtered);
                        auto_loaded = true;
                    }
                }

                if !auto_loaded {
                    if to_render < total_filtered {
                        ui.horizontal(|ui| {
                            if ui.button("Load more").clicked() {
                                self.sidebar_loaded_count = (to_render + self.sidebar_batch).min(total_filtered);
                            }
                        });
                    } else {
                        let _ = ui.ctx().input(|_i| {});
                    }
                }
            });
        });
    }
}
