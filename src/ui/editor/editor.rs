// Copyright (c) 2025 markdown-library

use crate::app::MarkdownApp;
use crate::editor_actions;
use crate::highlight;
use crate::settings::ViewMode;
use crate::ui::constants::EDITOR_ID;
use crate::ui::types::detect_list_marker;
use egui::{FontFamily, FontId, RichText, ScrollArea, TextEdit, Ui};

impl MarkdownApp {
    pub fn draw_editor(&mut self, ui: &mut Ui) {
        let Some(sel) = self.selected else {
            return;
        };

        let c = self.colors();
        let path_label = self.notes[sel]
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "[Unsaved]".to_string());

        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let note = &mut self.notes[sel];
                    let title_resp = ui.add(
                        TextEdit::singleline(&mut note.title)
                            .desired_width(220.0)
                            .frame(egui::Frame::NONE)
                            .font(FontId::new(14.0, FontFamily::Proportional))
                            .text_color(c.text_strong),
                    );
                    if title_resp.changed() {
                        note.modified = true;
                        note.touch();
                        self.notes_dirty = true;
                        if note.title_synced {
                            let mut lines: Vec<&str> = note.content.lines().collect();
                            if lines.is_empty() {
                                note.content = format!("# {}\n\n", note.title);
                            } else {
                                let mut replaced = false;
                                for i in 0..lines.len() {
                                    if i == 0 || !lines[i].trim().is_empty() {
                                        lines[i] = &*Box::leak(format!("# {}", note.title).into_boxed_str());
                                        replaced = true;
                                        break;
                                    }
                                }
                                if !replaced {
                                    lines.insert(0, &*Box::leak(format!("# {}", note.title).into_boxed_str()));
                                }
                                note.content = lines.join("\n");
                            }
                        }
                    }
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
                        ui.add_space(8.0);
                        ui.label(RichText::new(path_label).color(c.text_dim).size(11.0));
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("🏷").color(c.text_dim).size(11.0));
                    let note = &mut self.notes[sel];

                    let tag_id = egui::Id::new("tag_input");
                    let mut buffer = ui.ctx().data_mut(|d| {
                        if d.get_temp::<usize>(tag_id) != Some(sel) {
                            d.insert_temp(tag_id, sel);
                            note.tags.join(", ")
                        } else {
                            d.get_temp::<String>(tag_id).unwrap_or_default()
                        }
                    });

                    let resp = ui.add(
                        TextEdit::singleline(&mut buffer)
                            .hint_text("Add tags, comma separated...")
                            .desired_width(f32::INFINITY)
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .text_color(c.text_dim),
                    );

                    ui.ctx().data_mut(|d| { d.insert_temp(tag_id, buffer.clone()); });

                    if resp.changed() {
                        note.tags = buffer
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        note.touch();
                        self.notes_dirty = true;
                    }
                });
            });

        if self.settings.show_toolbar {
            self.draw_toolbar(ui);
        }

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        let font_size = self.settings.editor_font_size;
        let show_line_numbers = self.settings.show_line_numbers;
        let word_wrap = self.settings.word_wrap;

        let scroll = if word_wrap {
            ScrollArea::vertical()
        } else {
            ScrollArea::both()
        };

        scroll
            .id_salt("editor_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    if show_line_numbers {
                        let note = &self.notes[sel];
                        let line_count = note.content.lines().count().max(1);
                        let line_nums: String =
                            (1..=line_count).map(|n| format!("{}\n", n)).collect();
                        ui.add(
                            TextEdit::multiline(&mut line_nums.as_str())
                                .desired_width(36.0)
                                .frame(egui::Frame::NONE)
                                .interactive(false)
                                .font(FontId::new(font_size, FontFamily::Monospace))
                                .text_color(c.line_number),
                        );
                        ui.add(
                            egui::Separator::default().vertical().spacing(0.0).grow(0.0),
                        );
                    }

                    let note = &mut self.notes[sel];
                    let prev_content = note.content.clone();
                    let editor_id = egui::Id::new(EDITOR_ID);
                    let syntax_highlight = self.settings.syntax_highlight;
                    let theme_mode = self.settings.theme;
                    let text_color = c.text_normal;

                    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                        let mut job = if syntax_highlight {
                            highlight::layout_markdown(text.as_str(), font_size, theme_mode, text_color)
                        } else {
                            let mut j = egui::text::LayoutJob::default();
                            j.append(
                                text.as_str(),
                                0.0,
                                egui::text::TextFormat {
                                    font_id: FontId::new(font_size, FontFamily::Monospace),
                                    color: text_color,
                                    ..Default::default()
                                },
                            );
                            j
                        };
                        if word_wrap {
                            job.wrap.max_width = wrap_width;
                        }
                        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
                        galley
                    };

                    let mut editor = TextEdit::multiline(&mut note.content)
                        .id(editor_id)
                        .desired_rows(40)
                        .frame(egui::Frame::NONE)
                        .font(FontId::new(font_size, FontFamily::Monospace))
                        .text_color(c.text_normal)
                        .lock_focus(true)
                        .layouter(&mut layouter);
                    if word_wrap {
                        editor = editor.desired_width(f32::INFINITY);
                    } else {
                        editor = editor.desired_width(2000.0);
                    }
                    let output = editor.show(ui);

                    if note.content != prev_content {
                        note.modified = true;
                        let first_line = note.content.lines().next().unwrap_or("").trim().trim_start_matches('#').trim();
                        if first_line != note.title {
                            note.title_synced = false;
                        }
                        note.touch();
                        self.notes_dirty = true;
                    }

                    let enter_pressed = ui.ctx().input(|i| {
                        i.key_pressed(egui::Key::Enter)
                            && !i.modifiers.ctrl
                            && !i.modifiers.shift
                            && !i.modifiers.alt
                    });
                    if enter_pressed && output.response.has_focus() {
                        self.pending_list_continuation = true;
                    }

                    if let Some(action) = self.pending_action.take() {
                        let (sel_start, sel_end) = if let Some(range) = output.cursor_range {
                            (range.primary.index, range.secondary.index)
                        } else {
                            let end = note.content.chars().count();
                            (end, end)
                        };
                        let result =
                            editor_actions::apply(action, &note.content, sel_start, sel_end);
                        note.content = result.new_content;
                        note.modified = true;
                        note.touch();
                        self.notes_dirty = true;

                        let mut state = output.state.clone();
                        let new_range = egui::text::CCursorRange::two(
                            egui::text::CCursor::new(result.new_cursor_start),
                            egui::text::CCursor::new(result.new_cursor_end),
                        );
                        state.cursor.set_char_range(Some(new_range));
                        state.store(ui.ctx(), editor_id);
                        ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                    }

                    if let Some(up) = self.pending_line_move.take() {
                        let (sel_start, sel_end) = if let Some(range) = output.cursor_range {
                            (range.primary.index, range.secondary.index)
                        } else {
                            (0, 0)
                        };
                        if let Some(result) =
                            editor_actions::move_lines(&note.content, sel_start, sel_end, up)
                        {
                            note.content = result.new_content;
                            note.modified = true;
                            note.touch();
                            self.notes_dirty = true;

                            let mut state = output.state.clone();
                            let new_range = egui::text::CCursorRange::two(
                                egui::text::CCursor::new(result.new_cursor_start),
                                egui::text::CCursor::new(result.new_cursor_end),
                            );
                            state.cursor.set_char_range(Some(new_range));
                            state.store(ui.ctx(), editor_id);
                            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
                        }
                    }

                    if self.pending_list_continuation {
                        self.pending_list_continuation = false;
                        if let Some(range) = output.cursor_range {
                            let cursor = range.primary.index;
                            let chars: Vec<char> = note.content.chars().collect();
                            if cursor > 0 && cursor <= chars.len() && chars[cursor - 1] == '\n' {
                                let mut prev_start = cursor - 1;
                                while prev_start > 0 && chars[prev_start - 1] != '\n' {
                                    prev_start -= 1;
                                }
                                let prev_line: String = chars[prev_start..cursor - 1].iter().collect();
                                if let Some(marker) = detect_list_marker(&prev_line) {
                                    if marker.content_empty {
                                        let new_content: String = chars[..prev_start]
                                            .iter()
                                            .chain(chars[cursor..].iter())
                                            .collect();
                                        let new_pos = prev_start;
                                        note.content = new_content;
                                        let mut state = output.state.clone();
                                        let new_range = egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(new_pos),
                                            egui::text::CCursor::new(new_pos),
                                        );
                                        state.cursor.set_char_range(Some(new_range));
                                        state.store(ui.ctx(), editor_id);
                                    } else {
                                        let insertion: String = format!("{}{}", marker.indent, marker.next_marker);
                                        let new_content: String = chars[..cursor]
                                            .iter()
                                            .collect::<String>()
                                            + &insertion
                                            + &chars[cursor..].iter().collect::<String>();
                                        let new_pos = cursor + insertion.chars().count();
                                        note.content = new_content;
                                        let mut state = output.state.clone();
                                        let new_range = egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(new_pos),
                                            egui::text::CCursor::new(new_pos),
                                        );
                                        state.cursor.set_char_range(Some(new_range));
                                        state.store(ui.ctx(), editor_id);
                                    }
                                    note.modified = true;
                                    note.touch();
                                    self.notes_dirty = true;
                                }
                            }
                        }
                    }
                });
            });
    }
}
