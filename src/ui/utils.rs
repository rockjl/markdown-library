use crate::theme::{self, ThemeColors};
use crate::app::MarkdownApp;
use egui::{Color32, Ui};
use std::time::SystemTime;

impl MarkdownApp {
    pub fn paint_highlighted_text(
        &self,
        ui: &Ui,
        pos: egui::Pos2,
        text: &str,
        query_lc: &str,
        font: egui::FontId,
        normal_color: Color32,
        hl_color: Color32,
    ) {
        if query_lc.is_empty() {
            ui.painter().text(pos, egui::Align2::LEFT_TOP, text, font, normal_color);
            return;
        }

        let text_lc = text.to_lowercase();
        let mut x = pos.x;
        let y = pos.y;
        let mut idx = 0usize;
        while idx < text.len() {
            if let Some(rel) = text_lc[idx..].find(query_lc) {
                let start = idx + rel;
                if start > idx {
                    let pre = &text[idx..start];
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        pre,
                        0.0,
                        egui::text::TextFormat {
                            font_id: font.clone(),
                            color: normal_color,
                            ..Default::default()
                        },
                    );
                    let _galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
                    ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_TOP, pre, font.clone(), normal_color);
                    x += _galley.size().x;
                }

                let match_end = start + query_lc.len();
                let seg = &text[start..match_end];
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    seg,
                    0.0,
                    egui::text::TextFormat {
                        font_id: font.clone(),
                        color: hl_color,
                        ..Default::default()
                    },
                );
                let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
                ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_TOP, seg, font.clone(), hl_color);
                x += galley.size().x;

                idx = match_end;
            } else {
                let tail = &text[idx..];
                ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_TOP, tail, font.clone(), normal_color);
                break;
            }
        }
    }

    pub fn colors(&self) -> ThemeColors {
        theme::colors(self.settings.theme)
    }

    pub fn format_recent_time(&self, ts: i64) -> String {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let diff = now - ts;
        if diff < 0 {
            return "just now".to_string();
        }
        if diff < 60 {
            return format!("{}s ago", diff);
        }
        if diff < 3600 {
            return format!("{}m ago", diff / 60);
        }
        if diff < 86400 {
            return format!("{}h ago", diff / 3600);
        }
        if diff < 172800 {
            return "Yesterday".to_string();
        }
        if diff < 604800 {
            return format!("{}d ago", diff / 86400);
        }
        format!("{}w ago", diff / 604800)
    }

    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for n in &self.notes {
            if n.trashed { continue; }
            for t in &n.tags {
                if !t.is_empty() {
                    tags.insert(t.clone());
                }
            }
        }
        tags.into_iter().collect()
    }
}
