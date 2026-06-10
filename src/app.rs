use crate::attachments;
use crate::editor_actions::{self, EditorAction};
use crate::export;
use crate::search::index::SearchIndex;
use crate::find_replace::{self, FindReplaceState};
use crate::highlight;
use crate::note::Note;
use crate::settings::{FontChoice, Settings, ThemeMode, ViewMode};
use crate::storage;
use crate::theme::{self, ThemeColors};
    use crate::toc;
use crate::voice::VoiceEngine;
    use crate::wikilinks::{self, QuickSwitcherState};
    use crate::watcher::FSWatcher;
use egui::{Color32, FontFamily, FontId, RichText, ScrollArea, TextEdit, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::fs;
use std::collections::BTreeSet;
use std::time::Instant;

const EDITOR_ID: &str = "main_editor";
const AUTOSAVE_INTERVAL_SECS: u64 = 3;

    pub struct MarkdownApp {
        notes: Vec<Note>,
        selected: Option<usize>,
        library_mode: bool,
        cache: CommonMarkCache,
        search_history: Vec<String>,
        max_history: usize,
        selected_tags: Vec<String>,
        search_query: String,
        // Sidebar infinite-scroll state
        sidebar_loaded_count: usize,
        sidebar_batch: usize,
        sidebar_filter_key: String,
        sidebar_visible_indices: Vec<usize>,
        // Toolbar collapse state (in-memory)
        toolbar_collapsed: bool,
        // When set, the sidebar search box will request focus on next render
        focus_sidebar_search: bool,
    pending_action: Option<EditorAction>,
    pending_line_move: Option<bool>, // true = up, false = down
    pending_list_continuation: bool,
    settings: Settings,
    last_save_at: Instant,
    notes_dirty: bool,
    show_settings: bool,
    show_trash: bool,
    find: FindReplaceState,
    quick_switcher: QuickSwitcherState,
        show_backlinks: bool,
        // When opening backlinks via right-click, store the target note index so the
        // backlinks window can focus on that note's backlinks.
        current_backlinks_target: Option<usize>,
        // confirmation dialog state for restoring defaults
        confirm_restore_defaults: bool,
        // confirmation dialog for emptying trash
        confirm_empty_trash: bool,
        // IDs of notes created in this session (before first explicit save)
        new_note_ids: std::collections::HashSet<u64>,
        // optional filesystem watcher for content/ changes
        fs_watcher: Option<crate::watcher::FSWatcher>,
    status_override: Option<String>,
    status_override_at: Option<Instant>,
    voice_engine: Option<VoiceEngine>,
    voice_terminal: String,
    voice_search_results: Vec<crate::search::matcher::SearchHit>,
    voice_preview_mode: bool,
    current_transcript: String,
    search_index: Option<SearchIndex>,
}

impl MarkdownApp {

    // Helper: paint a single-line text with occurrences of `query_lc` highlighted.
    // - `pos` is the top-left position to start drawing
    // - `text` is the displayed text
    // - `query_lc` should be lowercase query; matching is done case-insensitively by lowercasing `text`
    // - `font` is the FontId to use
    // - `normal_color` and `hl_color` control colors
    fn paint_highlighted_text(
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
                // pre-match
                if start > idx {
                    let pre = &text[idx..start];
                    // measure width
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
                    let galley = ui.fonts(|f| f.layout_job(job));
                    ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_TOP, pre, font.clone(), normal_color);
                    x += galley.size().x;
                }

                // match segment
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
                let galley = ui.fonts(|f| f.layout_job(job));
                ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_TOP, seg, font.clone(), hl_color);
                x += galley.size().x;

                idx = match_end;
            } else {
                // trailing
                let tail = &text[idx..];
                ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_TOP, tail, font.clone(), normal_color);
                break;
            }
        }
    }
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let settings = storage::load_settings();
        load_user_font(&cc.egui_ctx, settings.font_choice);
        theme::apply(&cc.egui_ctx, settings.theme, settings.editor_font_size);

        let notes = storage::load_notes();
        let search_index = Some(storage::load_index()
            .unwrap_or_else(|| SearchIndex::build(&notes)));

        let mut app = Self {
            notes,
            selected: None,
            library_mode: false,
        cache: CommonMarkCache::default(),
            search_history: storage::load_search_history(),
            max_history: 50,
            selected_tags: Vec::new(),
            search_query: String::new(),
            sidebar_loaded_count: 0,
            sidebar_batch: settings.sidebar_batch,
            sidebar_filter_key: String::new(),
            sidebar_visible_indices: Vec::new(),
            toolbar_collapsed: settings.toolbar_collapsed,
            focus_sidebar_search: false,
            pending_action: None,
            pending_line_move: None,
            pending_list_continuation: false,
            settings,
            last_save_at: Instant::now(),
            notes_dirty: false,
            show_settings: false,
            show_trash: false,
            find: FindReplaceState::default(),
            quick_switcher: QuickSwitcherState::default(),
            show_backlinks: false,
            // spawn file watcher for content dir (non-blocking); created lazily in new
            fs_watcher: None,
            current_backlinks_target: None,
            confirm_restore_defaults: false,
            confirm_empty_trash: false,
            new_note_ids: std::collections::HashSet::new(),
            status_override: None,
            status_override_at: None,
            voice_engine: None,
            voice_terminal: String::new(),
            voice_search_results: Vec::new(),
            voice_preview_mode: false,
            current_transcript: String::new(),
            search_index,
        };
        // Start with no note selected — sidebar and editor both empty
        app.settings.view_mode = ViewMode::PreviewOnly;
        app
    }

    fn colors(&self) -> ThemeColors {
        theme::colors(self.settings.theme)
    }

    fn save_notes(&mut self) {
        storage::save_notes(&self.notes);
        // Update in-memory paths for newly saved notes so the sidebar
        // heuristic correctly identifies them as existing files.
        let content_dir = storage::content_dir();
        for n in self.notes.iter_mut() {
            if n.path.is_none() {
                n.path = Some(content_dir.join(format!("{}.md", n.id)));
            }
            n.modified = false;
        }
        // Rebuild search index from current notes and persist to disk
        self.search_index = Some(SearchIndex::build(&self.notes));
        if let Some(index) = &self.search_index {
            storage::save_index(index);
        }
        self.notes_dirty = false;
        self.last_save_at = Instant::now();
    }

    fn stop_and_search(&mut self) {
        let text = if let Some(ref eng) = self.voice_engine {
            eng.stop();
            eng.poll()
        } else {
            None
        };
        self.voice_engine = None;
        if let Some(transcript) = text {
            self.current_transcript = transcript.clone();
            if let Some(ref index) = self.search_index.clone() {
                let hits = crate::search::transcript_processor::process_transcript(index, &transcript);
                self.voice_search_results = hits;
                if !self.voice_search_results.is_empty() {
                    self.voice_preview_mode = true;
                    let best = self.voice_search_results[0].note_id;
                    if let Some(idx) = self.notes.iter().position(|n| n.id == best) {
                        self.selected = Some(idx);
                        self.settings.view_mode = ViewMode::PreviewOnly;
                    }
                }
            }
        }
    }

    fn save_settings(&self) {
        let mut s = self.settings.clone();
        // persist runtime toolbar_collapsed into settings before saving
        s.toolbar_collapsed = self.toolbar_collapsed;
        storage::save_settings(&s);
    }

    fn mark_dirty(&mut self) {
        self.notes_dirty = true;
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (
            ctrl,
            shift,
            alt,
            key_s,
            key_n,
            key_o,
            key_b,
            key_i,
            key_k,
            key_e,
            key_q,
            key_l,
            key_t,
            key_f,
            key_h,
            key_up,
            key_down,
            key_esc,
        ) = ctx.input(|i| {
            (
                i.modifiers.ctrl,
                i.modifiers.shift,
                i.modifiers.alt,
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::N),
                i.key_pressed(egui::Key::O),
                i.key_pressed(egui::Key::B),
                i.key_pressed(egui::Key::I),
                i.key_pressed(egui::Key::K),
                i.key_pressed(egui::Key::E),
                i.key_pressed(egui::Key::Q),
                i.key_pressed(egui::Key::L),
                i.key_pressed(egui::Key::T),
                i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::H),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Escape),
            )
        });

        if ctrl && key_s && !shift {
            if let Some(sel) = self.selected {
                let note_id = self.notes[sel].id;
                self.save_notes();
                if self.new_note_ids.remove(&note_id) {
                    self.settings.view_mode = ViewMode::PreviewOnly;
                }
            }
        }
        if ctrl && key_n && !shift {
            self.new_note();
        }
        if ctrl && key_o && !shift {
            self.open_file();
        }
        if ctrl && key_b && !shift {
            self.pending_action = Some(EditorAction::Wrap { prefix: "**", suffix: "**" });
        }
        if ctrl && key_i && !shift {
            self.pending_action = Some(EditorAction::Wrap { prefix: "*", suffix: "*" });
        }
        if ctrl && key_k && !shift {
            self.pending_action = Some(EditorAction::Insert("[リンクテキスト](https://)"));
        }
        if ctrl && key_e && !shift {
            self.pending_action = Some(EditorAction::Wrap { prefix: "`", suffix: "`" });
        }
        if ctrl && shift && key_q {
            self.pending_action = Some(EditorAction::LinePrefix("> "));
        }
        if ctrl && shift && key_l {
            self.pending_action = Some(EditorAction::LinePrefix("- "));
        }
        if ctrl && shift && key_t {
            self.pending_action = Some(EditorAction::LinePrefix("- [ ] "));
        }
        if ctrl && key_f && !shift {
            self.find.open_find();
        }
        if ctrl && key_h && !shift {
            self.find.open_replace();
        }
        if key_esc && self.find.visible {
            self.find.close();
        }
        if alt && key_up {
            self.pending_line_move = Some(true);
        }
        if alt && key_down {
            self.pending_line_move = Some(false);
        }
        // Navigate search results with up/down arrows
        if !ctrl && !alt && !shift && !self.sidebar_visible_indices.is_empty() {
            if key_up {
                if let Some(sel) = self.selected {
                    if let Some(pos) = self.sidebar_visible_indices.iter().position(|&i| i == sel) {
                        if pos > 0 {
                            let new_sel = self.sidebar_visible_indices[pos - 1];
                            self.selected = Some(new_sel);
                            if self.notes[new_sel].path.is_some() {
                                self.settings.view_mode = ViewMode::PreviewOnly;
                            } else {
                                self.settings.view_mode = ViewMode::EditorOnly;
                            }
                        }
                    }
                } else if let Some(&first) = self.sidebar_visible_indices.first() {
                    self.selected = Some(first);
                }
            }
            if key_down {
                if let Some(sel) = self.selected {
                    if let Some(pos) = self.sidebar_visible_indices.iter().position(|&i| i == sel) {
                        if pos + 1 < self.sidebar_visible_indices.len() {
                            let new_sel = self.sidebar_visible_indices[pos + 1];
                            self.selected = Some(new_sel);
                            if self.notes[new_sel].path.is_some() {
                                self.settings.view_mode = ViewMode::PreviewOnly;
                            } else {
                                self.settings.view_mode = ViewMode::EditorOnly;
                            }
                        }
                    }
                } else if let Some(&first) = self.sidebar_visible_indices.first() {
                    self.selected = Some(first);
                }
            }
        }
        // Image paste: TextEdit consumes Ctrl+V before we can see it, so
        // listen for the paste *event* itself, which still fires after the
        // When a paste happens, also check whether the clipboard carries an
        // image — if so, attach it.
        // We listen for BOTH Event::Paste (fires when clipboard has text) AND
        // raw Ctrl+V (needed for screenshots, which have no text in clipboard
        // and therefore never generate Event::Paste).
        let key_v = ctx.input(|i| i.key_pressed(egui::Key::V));
        let paste_event = ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Paste(_)))
        });
        if paste_event || (ctrl && !shift && key_v) {
            self.try_paste_image(false);
        }
        // Ctrl+Shift+V also works as an explicit trigger (e.g. when the
        // focus is not on the text editor).
        if ctrl && shift && key_v {
            self.try_paste_image(true);
        }
        // Ctrl+P: Quick switcher
        let key_p = ctx.input(|i| i.key_pressed(egui::Key::P));
        if ctrl && key_p && !shift {
            self.quick_switcher.open();
        }
        if key_esc && self.quick_switcher.visible {
            self.quick_switcher.close();
        }
        // Ctrl+, : switch to editor only
        let key_comma = ctx.input(|i| i.key_pressed(egui::Key::Comma));
        if ctrl && key_comma {
            self.settings.view_mode = ViewMode::EditorOnly;
            self.save_settings();
        }
        // Ctrl+. : switch to preview only
        let key_period = ctx.input(|i| i.key_pressed(egui::Key::Period));
        if ctrl && key_period {
            self.settings.view_mode = ViewMode::PreviewOnly;
            self.save_settings();
        }
        // Ctrl+/ : focus sidebar search input
        let key_slash = ctx.input(|i| i.key_pressed(egui::Key::Slash));
        if ctrl && key_slash {
            self.focus_sidebar_search = true;
        }

        // Ctrl+ArrowDown: select first note in current filtered sidebar list
        let key_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
        if ctrl && key_down && !shift {
            self.select_first_filtered_note();
        }

        // F12: Push-To-Talk for voice search
        let key_f12 = ctx.input(|i| i.key_pressed(egui::Key::F12));
        if key_f12 && !ctrl && !alt && !shift {
            if let Some(ref eng) = self.voice_engine {
                if eng.is_running() {
                    self.stop_and_search();
                }
            } else {
                let eng = VoiceEngine::start("c44dd50e", "971738b63e6eb7bfd94a8246648ca421");
                self.voice_engine = Some(eng);
            }
        }
    }

    fn select_first_filtered_note(&mut self) {
        let query = self.search_query.clone();
        let query_lc = query.to_lowercase();
        let show_trash = self.show_trash;
        for (i, note) in self.notes.iter().enumerate() {
            if note.trashed != show_trash {
                continue;
            }
            let search_match = query_lc.is_empty()
                || note.title.to_lowercase().contains(&query_lc)
                || note.content.to_lowercase().contains(&query_lc)
                || note.tags.iter().any(|t| t.contains(&query));
            let tags_match = if self.selected_tags.is_empty() {
                true
            } else {
                self.selected_tags.iter().all(|st| note.tags.iter().any(|t| t == st))
            };
            if search_match && tags_match {
                self.selected = Some(i);
                break;
            }
        }
    }

    fn toolbar_button(&mut self, ui: &mut Ui, label: &str, tooltip: &str, action: EditorAction) {
        let c = self.colors();
        let resp = ui.add(
            egui::Button::new(RichText::new(label).size(13.0).color(c.text_normal))
                .min_size(egui::vec2(32.0, 28.0))
                .fill(c.button_bg),
        );
        if resp.on_hover_text(tooltip).clicked() {
            self.pending_action = Some(action);
        }
    }

    fn draw_toolbar(&mut self, ui: &mut Ui) {
        let c = self.colors();
        // Toolbar area: full toolbar or a collapsed compact bar
        egui::Frame::none()
            .fill(c.toolbar_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                if self.toolbar_collapsed {
                    ui.horizontal(|ui| {
                        // compact: show only a few primary buttons and an expand toggle
                        self.toolbar_button(ui, "B", "Bold (Ctrl+B)", EditorAction::Wrap { prefix: "**", suffix: "**" });
                        self.toolbar_button(ui, "I", "Italic (Ctrl+I)", EditorAction::Wrap { prefix: "*", suffix: "*" });
                        self.toolbar_button(ui, "</>", "Inline code (Ctrl+E)", EditorAction::Wrap { prefix: "`", suffix: "`" });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("⤢").on_hover_text("Expand toolbar").clicked() {
                                self.toolbar_collapsed = false;
                                self.save_settings();
                            }
                        });
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.spacing_mut().item_spacing.y = 4.0;
                        self.toolbar_button(ui, "B", "Bold (Ctrl+B)", EditorAction::Wrap { prefix: "**", suffix: "**" });
                        self.toolbar_button(ui, "I", "Italic (Ctrl+I)", EditorAction::Wrap { prefix: "*", suffix: "*" });
                        self.toolbar_button(ui, "S", "Strikethrough", EditorAction::Wrap { prefix: "~~", suffix: "~~" });
                        ui.separator();
                        self.toolbar_button(ui, "H1", "Heading 1", EditorAction::LinePrefix("# "));
                        self.toolbar_button(ui, "H2", "Heading 2", EditorAction::LinePrefix("## "));
                        self.toolbar_button(ui, "H3", "Heading 3", EditorAction::LinePrefix("### "));
                        ui.separator();
                        self.toolbar_button(ui, "</>", "Inline code (Ctrl+E)", EditorAction::Wrap { prefix: "`", suffix: "`" });
                        self.toolbar_button(ui, "{ }", "Code block", EditorAction::CodeBlock(""));
                        self.toolbar_button(ui, "🔗", "Link (Ctrl+K)", EditorAction::Insert("[link](https://)"));
                        self.toolbar_button(ui, "🖼", "Image", EditorAction::Insert("![alt](path/to/image.png)"));
                        ui.separator();
                        self.toolbar_button(ui, "•", "Bullet list (Ctrl+Shift+L)", EditorAction::LinePrefix("- "));
                        self.toolbar_button(ui, "1.", "Numbered list", EditorAction::LinePrefix("1. "));
                        self.toolbar_button(ui, "☑", "Todo (Ctrl+Shift+T)", EditorAction::LinePrefix("- [ ] "));
                        self.toolbar_button(ui, "❝", "Quote (Ctrl+Shift+Q)", EditorAction::LinePrefix("> "));
                        self.toolbar_button(ui, "—", "Horizontal rule", EditorAction::Insert("\n---\n"));
                        self.toolbar_button(ui, "⊞", "Table", EditorAction::Table { rows: 2, cols: 3 });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("⤡").on_hover_text("Collapse toolbar").clicked() {
                                self.toolbar_collapsed = true;
                                self.save_settings();
                            }
                        });
                    });
                }
            });
    }

    fn save_current_to_file(&mut self) {
        // Save all notes to app-managed content/ storage using content/<id>.md
        // Use the existing save_notes helper so timestamps and dirty flags are consistent.
        self.save_notes();
        // Clear per-note modified flags so the UI doesn't show a trailing '*'
        for n in self.notes.iter_mut() {
            n.modified = false;
        }
    }

    fn open_file(&mut self) {
        // Open arbitrary markdown file into a new note (user-chosen file)
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file()
        else {
            return;
        };

        if let Ok(content) = fs::read_to_string(&path) {
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string();
            let mut note = Note::new(title, content);
            // Record the original path so the sidebar click heuristic correctly
            // identifies this as an existing file (path.is_some() → PreviewOnly).
            note.path = Some(path.clone());
            self.notes.push(note);
            self.selected = Some(self.notes.len() - 1);
            // Imported/existing files should open in preview by default
            self.settings.view_mode = ViewMode::PreviewOnly;
            self.mark_dirty();
        }
    }

    fn new_note(&mut self) {
        self.notes.push(Note::default());
        let idx = self.notes.len() - 1;
        self.selected = Some(idx);
        self.new_note_ids.insert(self.notes[idx].id);
        self.settings.view_mode = ViewMode::EditorOnly;
        self.mark_dirty();
    }

    fn move_to_trash(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes[idx].trashed = true;
        self.notes[idx].touch();
        if self.selected == Some(idx) {
            let next = self.notes.iter().position(|n| !n.trashed).unwrap_or(0);
            self.selected = Some(next);
        }
        self.mark_dirty();
    }

    fn restore_from_trash(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes[idx].trashed = false;
        self.notes[idx].touch();
        self.mark_dirty();
    }

    fn delete_permanently(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.new_note_ids.remove(&self.notes[idx].id);
        // Remove the on-disk file before removing from memory
        let content_dir = storage::content_dir();
        let path = content_dir.join(format!("{}.md", self.notes[idx].id));
        let _ = std::fs::remove_file(&path);
        self.notes.remove(idx);
        if self.selected.map_or(false, |s| s >= self.notes.len()) {
            self.selected = Some(self.notes.len().saturating_sub(1));
        }
        self.mark_dirty();
    }

    fn empty_trash(&mut self) {
        // Delete on-disk files and clean up new_note_ids for all trashed notes
        let content_dir = storage::content_dir();
        for n in self.notes.iter() {
            if n.trashed {
                self.new_note_ids.remove(&n.id);
                let path = content_dir.join(format!("{}.md", n.id));
                let _ = std::fs::remove_file(&path);
            }
        }
        self.notes.retain(|n| !n.trashed);
        if self.selected.map_or(false, |s| s >= self.notes.len()) {
            self.selected = Some(self.notes.len().saturating_sub(1));
        }
        self.mark_dirty();
    }

    fn toggle_star(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes[idx].starred = !self.notes[idx].starred;
        self.notes[idx].touch();
        self.mark_dirty();
    }

    fn ensure_valid_selection(&mut self) {
        let Some(sel) = self.selected else {
            // No note selected — leave it None, don't auto-select
            return;
        };
        let visible: Vec<usize> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.trashed == self.show_trash)
            .map(|(i, _)| i)
            .collect();

        if !visible.contains(&sel) {
            if let Some(&first) = visible.first() {
                self.selected = Some(first);
            } else {
                // No notes in current view — create one if we're in normal view
                if !self.show_trash {
                    self.notes.push(Note::default());
                    self.selected = Some(self.notes.len() - 1);
                    self.mark_dirty();
                }
            }
        }
    }

    fn ensure_fs_watcher(&mut self) {
        if self.fs_watcher.is_some() {
            return;
        }
        let path = storage::content_dir();
        if !path.exists() {
            return;
        }
        if let Ok(w) = FSWatcher::spawn(path) {
            self.fs_watcher = Some(w);
        }
    }

    fn draw_sidebar(&mut self, ui: &mut Ui) {
        let c = self.colors();

        // Header
        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let btn_size = egui::vec2(32.0, 32.0);

                    // Left: + new note
                    let resp = ui.add(
                        egui::Button::new(RichText::new("+").size(16.0).color(c.accent))
                            .min_size(btn_size)
                    );
                    if resp.on_hover_text("New note (Ctrl+N)").clicked() {
                        self.new_note();
                    }

                    // Left: 🎤 / 🔴 voice
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

                    // Right: mode toggle
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

        // Search
                egui::Frame::none()
                    .fill(c.sidebar_bg)
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                let search_resp = ui.add(
                    TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍  Search...")
                        .desired_width(f32::INFINITY)
                        .font(FontId::new(13.0, FontFamily::Proportional)),
                )
                .on_hover_text("Ctrl+/ to focus · Enter to search");
                // If focus was requested by a shortcut, request it now and clear the flag
                if self.focus_sidebar_search {
                    search_resp.request_focus();
                    self.focus_sidebar_search = false;
                }
                // Clear voice results when user types in search box
                if !self.voice_search_results.is_empty() && !self.search_query.is_empty() {
                    self.voice_search_results.clear();
                }

                    // Press Enter to run search and add to history
                    if search_resp.lost_focus() && ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                    let q = self.search_query.trim().to_string();
                    if !q.is_empty() {
                        // dedup and push to front
                        self.search_history.retain(|x| x != &q);
                        self.search_history.insert(0, q.clone());
                        if self.search_history.len() > self.max_history {
                            self.search_history.truncate(self.max_history);
                        }
                        storage::save_search_history(&self.search_history);
                    }
                    }

                // Quick-history dropdown
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

        // View tabs: All / Starred / Trash
        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
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
                            // Clicking the trash tab while already viewing trash → confirm clear
                            self.confirm_empty_trash = true;
                        } else {
                            self.show_trash = true;
                        }
                    }
                });
            });

        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        // Tag filter chips
        egui::Frame::none()
            .fill(c.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Tags:").color(c.text_dim).size(11.0));
                    // Show selected count and a clear button
                    if !self.selected_tags.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("Selected: {}", self.selected_tags.len())).color(c.text_dim).size(11.0));
                        if ui.small_button("Clear").on_hover_text("Clear selected tags").clicked() {
                            self.selected_tags.clear();
                        }
                    }
                    // collect all tags from notes (excluding trashed unless viewing trash)
                    let mut set: BTreeSet<String> = BTreeSet::new();
                    for (_i, n) in self.notes.iter().enumerate() {
                        if n.trashed != self.show_trash {
                            continue;
                        }
                        for t in &n.tags {
                            set.insert(t.clone());
                        }
                    }
                    for tag in set.iter() {
                        let selected = self.selected_tags.contains(tag);
                        // draw as a pill: background filled when selected, outlined otherwise
                        let (bg, fg) = if selected {
                            (c.accent, Color32::WHITE)
                        } else {
                            (c.toolbar_bg, c.text_normal)
                        };
                        let resp = ui.add(egui::Label::new(RichText::new(tag.clone()).color(fg)).sense(egui::Sense::click()));
                        let rect = resp.rect;
                        if rect.is_positive() {
                            // expand rect manually by padding
                            let pad = egui::vec2(8.0, 4.0);
                            let r = egui::Rect::from_min_max(rect.min - pad, rect.max + pad);
                            ui.painter().rect_filled(r, 6.0, bg);
                            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, tag.clone(), egui::FontId::new(11.0, FontFamily::Proportional), fg);
                        }
                        if resp.clicked() {
                            if selected {
                                self.selected_tags.retain(|t| t != tag);
                            } else {
                                self.selected_tags.push(tag.clone());
                            }
                        }
                        ui.add_space(6.0);
                    }
                });
            });

        // Note list
        egui::Frame::none().fill(c.sidebar_bg).show(ui, |ui| {
            ui.add_space(4.0);
            ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let query = self.search_query.clone();
                let query_lc = query.to_lowercase();
                let show_trash = self.show_trash;

                // Library mode: show all notes grouped by tags (when no search query)
                if self.library_mode && query_lc.is_empty() {
                    self.render_library_view(ui, show_trash);
                    return;
                }

                // Voice search results: show when available
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
                                self.settings.view_mode = ViewMode::PreviewOnly;
                            }
                        }
                        let bg = if is_selected {
                            c.selected_item_bg
                        } else if response.hovered() {
                            c.hover_item_bg
                        } else {
                            c.sidebar_bg
                        };
                        ui.painter().rect_filled(rect, 4.0, bg);
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

                // Build filtered index list using search module when query is present
                let filtered: Vec<usize> = if !query_lc.is_empty() {
                    if let Some(ref index) = self.search_index {
                        let hits = crate::search::matcher::search(index, &query, 0.0);
                        hits.into_iter()
                            .filter_map(|h| self.notes.iter().position(|n| n.id == h.note_id))
                            .filter(|&i| self.notes[i].trashed == show_trash)
                            .filter(|&i| {
                                let note = &self.notes[i];
                                self.selected_tags.is_empty()
                                    || self.selected_tags.iter().all(|st| note.tags.iter().any(|t| t == st))
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // If filter key changed (search text or tags or trash toggle), reset selection and loaded count
                let current_filter_key = format!("{}|{:?}|{}|{}", query_lc, self.selected_tags, show_trash, self.library_mode);
                if current_filter_key != self.sidebar_filter_key {
                    self.sidebar_filter_key = current_filter_key.clone();
                    self.sidebar_loaded_count = 0;
                    self.selected = None;
                }

                // ensure at least one batch loaded
                if self.sidebar_loaded_count == 0 {
                    self.sidebar_loaded_count = self.sidebar_batch.min(filtered.len());
                }

                // store all matched indices for keyboard navigation
                self.sidebar_visible_indices = filtered.clone();
                let total_filtered = filtered.len();
                let to_render = self.sidebar_loaded_count.min(total_filtered);
                let indices: Vec<usize> = filtered.into_iter().take(to_render).collect();

                // Auto-select first result when a search produced results and nothing is selected
                if !indices.is_empty() && self.selected.is_none() {
                    self.selected = Some(indices[0]);
                    self.settings.view_mode = ViewMode::PreviewOnly;
                }

                let mut new_selected = self.selected;
                let mut toggle_star_idx: Option<usize> = None;
                let mut trash_idx: Option<usize> = None;
                let mut restore_idx: Option<usize> = None;
                let mut delete_idx: Option<usize> = None;

                let mut last_item_rect: Option<egui::Rect> = None;
                for i in indices {
                    let is_selected = self.selected == Some(i);
                    let title = self.notes[i].display_title();
                    let starred = self.notes[i].starred;
                    let preview: String = self.notes[i]
                        .content
                        .lines()
                        .find(|l| !l.trim_start_matches('#').trim().is_empty())
                        .unwrap_or("")
                        .trim_start_matches('#')
                        .trim()
                        .chars()
                        .take(40)
                        .collect();

                    let desired_height = if preview.is_empty() { 36.0 } else { 52.0 };
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), desired_height),
                        egui::Sense::click(),
                    );

                    if response.clicked() {
                        new_selected = Some(i);
                    }

                    let bg = if is_selected {
                        c.selected_item_bg
                    } else if response.hovered() {
                        c.hover_item_bg
                    } else {
                        c.sidebar_bg
                    };
                    ui.painter().rect_filled(rect, 4.0, bg);

                    // Star icon
                    let star_rect = egui::Rect::from_min_size(
                        rect.right_top() + egui::vec2(-30.0, 8.0),
                        egui::vec2(20.0, 20.0),
                    );
                    let star_resp = ui.interact(
                        star_rect,
                        egui::Id::new(("star", i)),
                        egui::Sense::click(),
                    );
                    let star_color = if starred {
                        Color32::from_rgb(255, 200, 60)
                    } else {
                        c.text_dim
                    };
                    ui.painter().text(
                        star_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        if starred { "★" } else { "☆" },
                        egui::FontId::new(14.0, FontFamily::Proportional),
                        star_color,
                    );
                    if star_resp.clicked() {
                        toggle_star_idx = Some(i);
                    }

                    let title_color = if is_selected {
                        Color32::WHITE
                    } else {
                        c.text_normal
                    };
                    let title_pos = rect.min + egui::vec2(12.0, 10.0);
                    // highlight search matches in title (case-insensitive for title)
                    let title_font = egui::FontId::new(13.0, FontFamily::Proportional);
                    self.paint_highlighted_text(
                        ui,
                        title_pos,
                        &title,
                        &query_lc,
                        title_font,
                        title_color,
                        Color32::from_rgb(255, 220, 120),
                    );

                    if !preview.is_empty() {
                        // Highlight search query matches in preview/title (simple coloring)
                        let preview_color = if is_selected {
                            Color32::from_rgb(220, 245, 235)
                        } else {
                            c.text_dim
                        };
                        let preview_pos = rect.min + egui::vec2(12.0, 28.0);
                        let preview_font = egui::FontId::new(11.0, FontFamily::Proportional);
                        self.paint_highlighted_text(
                            ui,
                            preview_pos,
                            &preview,
                            &query_lc,
                            preview_font,
                            preview_color,
                            Color32::from_rgb(255, 220, 120),
                        );
                    }

                    // Right-click context menu
                    response.context_menu(|ui| {
                        if !show_trash {
                        if ui.button(if starred { "★ Unfavorite" } else { "☆ Add to favorites" }).clicked() {
                            toggle_star_idx = Some(i);
                            ui.close_menu();
                        }
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
                    if let Some(sel) = self.selected {
                        if self.library_mode {
                            self.settings.view_mode = ViewMode::EditorOnly;
                        } else if self.notes[sel].path.is_some() {
                            self.settings.view_mode = ViewMode::PreviewOnly;
                        } else {
                            self.settings.view_mode = ViewMode::EditorOnly;
                        }
                    }
                }
                if let Some(i) = toggle_star_idx {
                    self.toggle_star(i);
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
                // Auto-load more when the last rendered item is visible near the bottom of the viewport,
                // otherwise show a stable "Load more" button as a fallback.
                let mut auto_loaded = false;
                if let Some(r) = last_item_rect {
                    // clip rect is the visible viewport area
                    let clip_max_y = ui.clip_rect().max.y;
                    // if the last item's bottom is within 48px of the viewport bottom, load another batch
                    // If the last item's bottom is within a small threshold of the viewport bottom,
                    // load another batch. Use a smaller threshold for more precise "reach-bottom" detection.
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
                        // consume input ctx closure without using the arg to avoid unused-variable warning
                        let _ = ui.ctx().input(|_i| {});
                    }
                }
            });
        });
    }

    fn render_library_view(&mut self, ui: &mut Ui, show_trash: bool) {
        let c = self.colors();

        let mut tag_map: std::collections::BTreeMap<String, Vec<usize>> = std::collections::BTreeMap::new();
        let mut uncategorized: Vec<usize> = Vec::new();

        for (i, note) in self.notes.iter().enumerate() {
            if note.trashed != show_trash {
                continue;
            }
            if note.tags.is_empty() {
                uncategorized.push(i);
            } else {
                for tag in &note.tags {
                    tag_map.entry(tag.clone()).or_default().push(i);
                }
            }
        }

        for v in tag_map.values_mut() {
            v.sort_by_key(|&i| std::cmp::Reverse(self.notes[i].updated_at));
            v.dedup();
        }
        uncategorized.sort_by_key(|&i| std::cmp::Reverse(self.notes[i].updated_at));
        uncategorized.dedup();

        // Collect all note indices in display order for keyboard navigation
        let mut all_indices: Vec<usize> = Vec::new();

        for (tag, indices) in &tag_map {
            all_indices.extend(indices.iter().copied());
            let group_count = indices.len();
            let header = format!("{}  ( {})", tag, group_count);
            let id = ui.make_persistent_id(format!("tag_group_{}", tag));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    ui.label(RichText::new(&header).color(c.text_normal).size(12.0).strong());
                })
                .body(|ui| {
                    ui.indent(id, |ui| {
                        for &i in indices {
                            self.render_note_item(ui, i, c);
                        }
                    });
                });
        }

        if !uncategorized.is_empty() {
            all_indices.extend(uncategorized.iter().copied());
            let header = format!("Uncategorized  ( {})", uncategorized.len());
            let id = ui.make_persistent_id("tag_group_uncategorized");
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    ui.label(RichText::new(&header).color(c.text_dim).size(12.0));
                })
                .body(|ui| {
                    ui.indent(id, |ui| {
                        for &i in &uncategorized {
                            self.render_note_item(ui, i, c);
                        }
                    });
                });
        }

        self.sidebar_visible_indices = all_indices;
    }

    fn render_note_item(&mut self, ui: &mut Ui, i: usize, c: ThemeColors) {
        let is_selected = self.selected == Some(i);
        let title = self.notes[i].display_title();
        let starred = self.notes[i].starred;
        let preview: String = self.notes[i]
            .content
            .lines()
            .find(|l| !l.trim_start_matches('#').trim().is_empty())
            .unwrap_or("")
            .trim_start_matches('#')
            .trim()
            .chars()
            .take(40)
            .collect();

        let desired_height = if preview.is_empty() { 36.0 } else { 52.0 };
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), desired_height),
            egui::Sense::click(),
        );

        if response.clicked() {
            self.selected = Some(i);
            if self.library_mode {
                self.settings.view_mode = ViewMode::EditorOnly;
            } else if self.notes[i].path.is_some() {
                self.settings.view_mode = ViewMode::PreviewOnly;
            } else {
                self.settings.view_mode = ViewMode::EditorOnly;
            }
        }

        let bg = if is_selected {
            c.selected_item_bg
        } else if response.hovered() {
            c.hover_item_bg
        } else {
            c.sidebar_bg
        };
        ui.painter().rect_filled(rect, 4.0, bg);

        // Star icon
        let star_rect = egui::Rect::from_min_size(
            rect.right_top() + egui::vec2(-30.0, 8.0),
            egui::vec2(20.0, 20.0),
        );
        let star_resp = ui.interact(
            star_rect,
            egui::Id::new(("star", i)),
            egui::Sense::click(),
        );
        let star_color = if starred {
            Color32::from_rgb(255, 200, 60)
        } else {
            c.text_dim
        };
        ui.painter().text(
            star_rect.center(),
            egui::Align2::CENTER_CENTER,
            if starred { "★" } else { "☆" },
            egui::FontId::new(14.0, FontFamily::Proportional),
            star_color,
        );
        if star_resp.clicked() {
            self.toggle_star(i);
        }

        let title_color = if is_selected {
            Color32::WHITE
        } else {
            c.text_normal
        };
        let title_pos = rect.min + egui::vec2(12.0, 10.0);
        ui.painter().text(
            title_pos,
            egui::Align2::LEFT_TOP,
            &title,
            egui::FontId::new(13.0, FontFamily::Proportional),
            title_color,
        );

        if !preview.is_empty() {
            let preview_color = if is_selected {
                Color32::from_rgb(220, 245, 235)
            } else {
                c.text_dim
            };
            let preview_pos = rect.min + egui::vec2(12.0, 28.0);
            ui.painter().text(
                preview_pos,
                egui::Align2::LEFT_TOP,
                &preview,
                egui::FontId::new(11.0, FontFamily::Proportional),
                preview_color,
            );
        }

        // Right-click context menu
        let show_trash = self.show_trash;
        response.context_menu(|ui| {
            if !show_trash {
                if ui.button(if starred { "★ Unfavorite" } else { "☆ Add to favorites" }).clicked() {
                    self.toggle_star(i);
                    ui.close_menu();
                }
                if ui.button("🔎 Show Backlinks").clicked() {
                    self.selected = Some(i);
                    self.current_backlinks_target = Some(i);
                    self.show_backlinks = true;
                    ui.close_menu();
                }
                if ui.button("🗑 Move to Trash").clicked() {
                    self.move_to_trash(i);
                    ui.close_menu();
                }
            } else {
                if ui.button("↩ Restore").clicked() {
                    self.restore_from_trash(i);
                    ui.close_menu();
                }
                if ui.button("❌ Delete permanently").clicked() {
                    self.delete_permanently(i);
                    ui.close_menu();
                }
            }
        });
    }

    fn draw_editor(&mut self, ui: &mut Ui) {
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
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Title editor (single-line)
                    let note = &mut self.notes[sel];
                    let title_resp = ui.add(
                        TextEdit::singleline(&mut note.title)
                            .desired_width(220.0)
                            .frame(false)
                            .font(FontId::new(14.0, FontFamily::Proportional))
                            .text_color(c.text_strong),
                    );
                    if title_resp.changed() {
                        note.modified = true;
                        note.touch();
                        self.notes_dirty = true;
                        // If this note is synced-from-title (new note), update first heading
                        if note.title_synced {
                            // Update first heading line in content to match title
                            let mut lines: Vec<&str> = note.content.lines().collect();
                            if lines.is_empty() {
                                note.content = format!("# {}\n\n", note.title);
                            } else {
                                // Replace first non-empty line or the very first line
                                let mut replaced = false;
                                for i in 0..lines.len() {
                                    if i == 0 || !lines[i].trim().is_empty() {
                                        // ensure it starts with '# '
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
                        // View mode toggle buttons (right side)
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

                // Tag editing row
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🏷").color(c.text_dim).size(11.0));
                    let note = &mut self.notes[sel];
                    let mut tags_joined = note.tags.join(", ");
                    let resp = ui.add(
                        TextEdit::singleline(&mut tags_joined)
                            .hint_text("Add tags, comma separated...")
                            .desired_width(f32::INFINITY)
                            .font(FontId::new(11.0, FontFamily::Proportional))
                            .text_color(c.text_dim),
                    );
                    if resp.changed() {
                        note.tags = tags_joined
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
                                .frame(false)
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

                    let mut layouter = |ui: &Ui, text: &str, wrap_width: f32| {
                        let mut job = if syntax_highlight {
                            highlight::layout_markdown(text, font_size, theme_mode, text_color)
                        } else {
                            let mut j = egui::text::LayoutJob::default();
                            j.append(
                                text,
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
                        ui.fonts(|f| f.layout_job(job))
                    };

                    let mut editor = TextEdit::multiline(&mut note.content)
                        .id(editor_id)
                        .desired_rows(40)
                        .frame(false)
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

                    // If user edits the first heading line directly, disable title_synced
                    if note.content != prev_content {
                        note.modified = true;
                        // check if the first heading was edited by the user (different from title)
                        let first_line = note.content.lines().next().unwrap_or("").trim().trim_start_matches('#').trim();
                        if first_line != note.title {
                            note.title_synced = false;
                        }
                        note.touch();
                        self.notes_dirty = true;
                    }

                    // Detect Enter for list continuation
                    let enter_pressed = ui.ctx().input(|i| {
                        i.key_pressed(egui::Key::Enter)
                            && !i.modifiers.ctrl
                            && !i.modifiers.shift
                            && !i.modifiers.alt
                    });
                    if enter_pressed && output.response.has_focus() {
                        self.pending_list_continuation = true;
                    }

                    // Apply pending markdown action (toolbar/shortcut)
                    if let Some(action) = self.pending_action.take() {
                        let (sel_start, sel_end) = if let Some(range) = output.cursor_range {
                            (range.primary.ccursor.index, range.secondary.ccursor.index)
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

                    // Apply line move (Alt+Up/Down)
                    if let Some(up) = self.pending_line_move.take() {
                        let (sel_start, sel_end) = if let Some(range) = output.cursor_range {
                            (range.primary.ccursor.index, range.secondary.ccursor.index)
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

                    // Apply list continuation (Enter pressed)
                    if self.pending_list_continuation {
                        self.pending_list_continuation = false;
                        // Cursor is now after the inserted newline; we need to handle
                        // the case AFTER egui inserted the '\n'. Re-derive cursor.
                        if let Some(range) = output.cursor_range {
                            let cursor = range.primary.ccursor.index;
                            // We want to inspect the line that ended at cursor-1 (the just-broken line).
                            // After egui inserts \n, the previous line is the marker line.
                            // We need to look at line BEFORE cursor.
                            let chars: Vec<char> = note.content.chars().collect();
                            if cursor > 0 && cursor <= chars.len() && chars[cursor - 1] == '\n' {
                                // Find prev line start
                                let mut prev_start = cursor - 1;
                                while prev_start > 0 && chars[prev_start - 1] != '\n' {
                                    prev_start -= 1;
                                }
                                let prev_line: String = chars[prev_start..cursor - 1].iter().collect();
                                if let Some(marker) = detect_list_marker(&prev_line) {
                                    if marker.content_empty {
                                        // Remove the marker on prev line AND the newline (exit list)
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
                                        // Insert marker at cursor
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

    fn draw_preview(&mut self, ui: &mut Ui) {
        let Some(sel) = self.selected else { return; };
        let c = self.colors();
        let raw = &self.notes[sel].content;
        let content = wikilinks::render_for_preview(raw);

        egui::Frame::none().fill(c.preview_bg).show(ui, |ui| {
            egui::Frame::none()
                .fill(c.header_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
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
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width());
                            CommonMarkViewer::new()
                                .max_image_width(Some(600))
                                .show(ui, &mut self.cache, &content);
                        });
                });
        });
    }

    fn draw_settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut open = self.show_settings;
        let mut settings_changed = false;
        let mut font_changed = false;
        egui::Window::new("⚙ Settings")
            .open(&mut open)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Appearance").strong().size(14.0));
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        if ui
                            .selectable_label(self.settings.theme == ThemeMode::Dark, "🌙 Dark")
                            .clicked()
                        {
                            self.settings.theme = ThemeMode::Dark;
                            settings_changed = true;
                        }
                        if ui
                            .selectable_label(self.settings.theme == ThemeMode::Light, "☀ Light")
                            .clicked()
                        {
                            self.settings.theme = ThemeMode::Light;
                            settings_changed = true;
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(RichText::new("Editor").strong().size(14.0));

                    ui.horizontal(|ui| {
                        ui.label("Font:");
                        let current = self.settings.font_choice;
                        egui::ComboBox::from_id_salt("font_choice")
                            .selected_text(current.display_name())
                            .show_ui(ui, |ui| {
                                for &choice in FontChoice::all() {
                                    if ui
                                        .selectable_label(current == choice, choice.display_name())
                                        .clicked()
                                    {
                                        self.settings.font_choice = choice;
                                        font_changed = true;
                                        settings_changed = true;
                                    }
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Font size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.settings.editor_font_size)
                                    .range(8.0..=32.0)
                                    .speed(0.5)
                                    .suffix(" px"),
                            )
                            .changed()
                        {
                            settings_changed = true;
                        }
                    });
                    if ui
                        .checkbox(&mut self.settings.show_line_numbers, "Show line numbers")
                        .changed()
                    {
                        settings_changed = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.word_wrap, "Word wrap")
                        .changed()
                    {
                        settings_changed = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.auto_save, "Auto save")
                        .changed()
                    {
                        settings_changed = true;
                    }
                    if ui
                        .checkbox(&mut self.settings.syntax_highlight, "Syntax highlight")
                        .changed()
                    {
                        settings_changed = true;
                    }
                });

                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Storage").strong().size(14.0));
                    ui.horizontal(|ui| {
                        ui.label("Data directory:");
                        ui.monospace(storage::data_dir().display().to_string());
                    });

                    ui.horizontal(|ui| {
                        if ui.button("📂 Open data folder").clicked() {
                            let path = storage::data_dir();
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("explorer").arg(&path).spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open").arg(&path).spawn();
                            }
                            #[cfg(all(unix, not(target_os = "macos")))]
                            {
                                let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                            }
                        }


                    });
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("Restore Defaults")
                        .on_hover_text("Reset all settings to default values")
                        .clicked()
                    {
                        self.confirm_restore_defaults = true;
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new("Advanced").strong().size(14.0));
                ui.horizontal(|ui| {
                    ui.label("Sidebar batch size:");
                    let mut b = self.sidebar_batch as i32;
                    if ui.add(egui::DragValue::new(&mut b).range(8..=500)).changed() {
                        self.sidebar_batch = b.max(8) as usize;
                        settings_changed = true;
                    }
                });
                if ui
                    .checkbox(&mut self.settings.show_toolbar, "Show toolbar")
                    .on_hover_text("Toggle toolbar visibility")
                    .changed()
                {
                    settings_changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Sidebar width:");
                    let mut w = self.settings.sidebar_width;
                    if ui.add(egui::DragValue::new(&mut w).range(160.0..=800.0)).changed() {
                        self.settings.sidebar_width = w.max(160.0);
                        settings_changed = true;
                    }
                });
            });
        self.show_settings = open;
        if font_changed {
            load_user_font(ctx, self.settings.font_choice);
        }
        if settings_changed {
            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
            self.save_settings();
        }

        // Confirmation dialog for Restore Defaults
        if self.confirm_restore_defaults {
            egui::Window::new("Confirm Restore Defaults")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("This will reset all settings to defaults. Continue?");
                    ui.horizontal(|ui| {
                        if ui.button("Yes, restore").clicked() {
                            self.settings = crate::settings::Settings::default();
                            self.toolbar_collapsed = self.settings.toolbar_collapsed;
                            theme::apply(ctx, self.settings.theme, self.settings.editor_font_size);
                            self.save_settings();
                            self.confirm_restore_defaults = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_restore_defaults = false;
                        }
                    });
                });
        }
    }

    fn draw_find_bar(&mut self, ctx: &egui::Context) {
        if !self.find.visible {
            return;
        }
        let Some(sel) = self.selected else { return; };
        let c = self.colors();
        egui::TopBottomPanel::top("find_bar")
            .frame(
                egui::Frame::none()
                    .fill(c.toolbar_bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
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

                    // Match count
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

        // Enter key while focused on the search bar jumps to next match
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

    fn jump_to_match(&self, ctx: &egui::Context, matches: &[(usize, usize)]) {
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

    fn replace_current_match(&mut self) {
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

    fn replace_all(&mut self) {
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

    fn select_note_by_index(&mut self, idx: usize) {
        if idx < self.notes.len() {
            self.selected = Some(idx);
        }
    }

    fn navigate_to_wikilink(&mut self, target: &str) {
        let matches = wikilinks::resolve(&self.notes, target);
        if let Some(&idx) = matches.first() {
            self.selected = Some(idx);
        } else {
            let mut note = Note::new(target.to_string(), format!("# {}\n\n", target));
            note.modified = true;
            self.notes.push(note);
            self.selected = Some(self.notes.len() - 1);
            self.notes_dirty = true;
        }
    }

    fn draw_backlinks_panel(&mut self, ui: &mut Ui) {
        let Some(sel) = self.selected else { return; };
        let c = self.colors();
        let current_title = self.notes[sel].title.clone();

        let backlinks_index = wikilinks::build_backlink_index(&self.notes);
        let target_idx = self.current_backlinks_target.unwrap_or(sel);
        let backlinks = backlinks_index.get(&target_idx).cloned().unwrap_or_default();
        let outgoing = wikilinks::extract(&self.notes[target_idx].content);
        

        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
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

                // Outgoing links
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
        // Once we've opened the backlinks window for a specific target triggered by right-click,
        // clear the transient target so subsequent opens via menu use the current selection.
        self.current_backlinks_target = None;
    }

    fn draw_quick_switcher(&mut self, ctx: &egui::Context) {
        if !self.quick_switcher.visible {
            return;
        }
        let c = self.colors();
        let mut close = false;
        let mut navigate_to: Option<usize> = None;

        // Build filtered candidates
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

        // Key navigation
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
            // Create new note with the query as title
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
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .default_width(480.0)
            .frame(
                egui::Frame::popup(&ctx.style())
                    .fill(c.toolbar_bg)
                    .rounding(8.0)
                    .inner_margin(egui::Margin::same(10.0)),
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
                            .frame(false),
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
                            egui::Color32::TRANSPARENT
                        };
                        let resp = egui::Frame::none()
                            .fill(bg)
                            .rounding(4.0)
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(if note.starred { "★ " } else { "  " })
                                            .color(egui::Color32::from_rgb(255, 200, 60)),
                                    );
                                    ui.label(
                                        RichText::new(&note.title)
                                            .color(if is_active {
                                                egui::Color32::WHITE
                                            } else {
                                                c.text_normal
                                            })
                                            .size(13.0),
                                    );
                                });
                            })
                            .response
                            .interact(egui::Sense::click());
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

    fn try_paste_image(&mut self, force_message: bool) {
        let result = attachments::paste_clipboard_image();
        match result {
            Ok(Some(path)) => {
                let md = attachments::markdown_link_for(&path);
                let leaked: &'static str = Box::leak(md.into_boxed_str());
                self.pending_action = Some(EditorAction::Insert(leaked));
                self.status_override = Some(format!(
                    "Pasted image: {}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("image.png")
                ));
                self.status_override_at = Some(Instant::now());
            }
            Ok(None) => {
                    if force_message {
                        self.status_override = Some("No image in clipboard".to_string());
                        self.status_override_at = Some(Instant::now());
                    }
            }
            Err(msg) => {
                self.status_override = Some(format!("Failed to paste image: {}", msg));
                self.status_override_at = Some(Instant::now());
            }
        }
    }

    fn export_html(&self) {
        let Some(sel) = self.selected else { return; };
        let note = &self.notes[sel];
        let Some(path) = rfd::FileDialog::new()
            .add_filter("HTML", &["html", "htm"])
            .set_file_name(format!("{}.html", note.title))
            .save_file()
        else {
            return;
        };
        let html = export::markdown_to_html(&note.content, &note.title);
        let _ = std::fs::write(path, html);
    }

    fn draw_toc(&mut self, ui: &mut Ui) {
        let Some(sel) = self.selected else { return; };
        let c = self.colors();
        egui::Frame::none()
            .fill(c.header_bg)
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            .show(ui, |ui| {
                ui.label(RichText::new("📑  TOC").color(c.text_dim).size(12.0));
            });
        ui.add(egui::Separator::default().spacing(0.0).grow(0.0));

        let content = self.notes[sel].content.clone();
        let headings = toc::extract(&content);

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

    fn auto_save_if_needed(&mut self) {
        if self.settings.auto_save
            && self.notes_dirty
            && self.last_save_at.elapsed().as_secs() >= AUTOSAVE_INTERVAL_SECS
        {
            self.save_notes();
        }
    }
}

enum NavTarget {
    Wiki(String),
    Index(usize),
}

struct ListMarkerInfo {
    indent: String,
    next_marker: String,
    content_empty: bool,
}

fn detect_list_marker(line: &str) -> Option<ListMarkerInfo> {
    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let after = &line[indent.len()..];

    if let Some(rest) = after.strip_prefix("- [ ] ").or_else(|| after.strip_prefix("- [x] ")).or_else(|| after.strip_prefix("- [X] ")) {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "- [ ] ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("- ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "- ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("* ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "* ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("+ ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "+ ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("> ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "> ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    // Numbered: "N. "
    let digit_end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 {
        let after_digits = &after[digit_end..];
        if let Some(rest) = after_digits.strip_prefix(". ") {
            let n: u32 = after[..digit_end].parse().unwrap_or(0);
            return Some(ListMarkerInfo {
                indent,
                next_marker: format!("{}. ", n + 1),
                content_empty: rest.trim().is_empty(),
            });
        }
    }
    None
}

fn load_user_font(ctx: &egui::Context, font_choice: FontChoice) {
    // SystemDefault uses egui's built-in fonts (good English fonts out of the box).
    if matches!(font_choice, FontChoice::SystemDefault) {
        ctx.set_fonts(egui::FontDefinitions::default());
        return;
    }

    let mut fonts = egui::FontDefinitions::default();
    let selected = font_choice.font_candidates();
    let font_name = font_choice.display_name();

    use std::collections::HashSet;
    let mut tried: HashSet<String> = HashSet::new();

    let mut try_path = |path: &str| -> bool {
        if tried.contains(path) {
            return false;
        }
        tried.insert(path.to_string());
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                font_name.to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(font_name.to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(font_name.to_owned());
            return true;
        }
        false
    };

    for &p in selected.iter() {
        if try_path(p) {
            ctx.set_fonts(fonts);
            return;
        }
    }

    // Fallback: try common English font paths on each OS
    let common_fallbacks: &[&str] = &[
        // Windows
        r"C:\Windows\Fonts\SegoeUI.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\arial.ttf",
        // macOS
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Arial.ttf",
        // Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    for &p in common_fallbacks.iter() {
        if try_path(p) {
            ctx.set_fonts(fonts);
            return;
        }
    }

    ctx.set_fonts(fonts);
}

impl eframe::App for MarkdownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.ensure_valid_selection();
        // lazily create a filesystem watcher for the content directory
        self.ensure_fs_watcher();
        // Poll filesystem watcher for changes and reload notes when safe
        if let Some(w) = &self.fs_watcher {
            // drain all pending events
            while let Some(ev_res) = w.try_recv() {
                match ev_res {
                    Ok(_ev) => {
                        // If a watcher event arrives immediately after we saved files,
                        // it is likely our own write causing the event. To avoid
                        // unnecessary reloads (which can confuse UI state), ignore
                        // watcher events that occur within a short debounce window
                        // after our last_save_at timestamp.
                        if self.last_save_at.elapsed().as_millis() < 1500 {
                            // skip this event as it's probably from our own save
                            continue;
                        }

                        // If we have no unsaved local changes, reload notes from disk.
                        if !self.notes_dirty && self.notes.iter().all(|n| !n.modified) {
                            let old_selected_title = self.selected.and_then(|s| self.notes.get(s)).map(|n| n.title.clone());
                            let new_notes = storage::load_notes();
                            if !new_notes.is_empty() {
                                self.notes = new_notes;
                                // also reload search index
                                if let Some(idx) = storage::load_index() {
                                    self.search_index = Some(idx);
                                } else {
                                    self.search_index = Some(SearchIndex::build(&self.notes));
                                }
                                // try to restore selection by title, fallback to valid selection
                                if let Some(title) = old_selected_title {
                                    if let Some(idx) = self.notes.iter().position(|n| n.title == title) {
                                        self.selected = Some(idx);
                                    }
                                }
                                self.status_override = Some("Notes reloaded due to external change".to_string());
                                self.status_override_at = Some(Instant::now());
                            }
                        } else {
                            // notify user that external changes were detected but local edits exist
                            self.status_override = Some("External changes detected; not reloaded due to unsaved edits".to_string());
                            self.status_override_at = Some(Instant::now());
                        }
                    }
                    Err(_) => {
                        // ignore watcher errors but notify briefly
                        self.status_override = Some("Filesystem watch error".to_string());
                        self.status_override_at = Some(Instant::now());
                    }
                }
            }
        }
        self.auto_save_if_needed();
        let c = self.colors();

        // Menu bar
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(c.menu_bg)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
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

        // Status bar
        let dirty = self.notes_dirty;
        let auto_save = self.settings.auto_save;
        // Clear status override after 6 seconds
        if let Some(at) = self.status_override_at {
            if at.elapsed().as_secs() >= 6 {
                self.status_override = None;
                self.status_override_at = None;
            }
        }
        let status_override = self.status_override.clone();
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none().fill(c.menu_bg).inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut status_parts = Vec::new();
                    // Recording indicator
                    if self.voice_engine.as_ref().map_or(false, |v| v.is_running()) {
                        if !self.current_transcript.is_empty() {
                            status_parts.push(format!("🎤 Rec · \"{}\"", self.current_transcript));
                        } else {
                            status_parts.push("🎤 Recording...".to_string());
                        }
                    }
                    // Status override or dirty/clean
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

        // Sidebar
        if self.settings.show_sidebar {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .min_width(180.0)
                .default_width(220.0)
                .frame(egui::Frame::none().fill(c.sidebar_bg))
                .show(ctx, |ui| {
                    self.draw_sidebar(ui);
                });
        }

        // Backlinks: show as a pop-up window instead of fixed bottom panel
        if self.show_backlinks {
            let mut open = self.show_backlinks;
            egui::Window::new("Links")
                .open(&mut open)
                .resizable(true)
                .default_height(220.0)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -80.0])
                .frame(egui::Frame::none().fill(c.sidebar_bg))
                .show(ctx, |ui| {
                    self.draw_backlinks_panel(ui);
                });
            self.show_backlinks = open;
        }

        // TOC: show as a pop-up window instead of fixed side panel
        if self.settings.show_toc {
            let mut open_toc = self.settings.show_toc;
            egui::Window::new("Table of Contents")
                .open(&mut open_toc)
                .resizable(true)
                .default_width(320.0)
                .anchor(egui::Align2::CENTER_TOP, [-80.0, 0.0])
                .frame(egui::Frame::none().fill(c.sidebar_bg))
                .show(ctx, |ui| {
                    self.draw_toc(ui);
                });
            self.settings.show_toc = open_toc;
        }

        // Main area
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| match self.settings.view_mode {
                ViewMode::Split => {
                    ui.columns(2, |cols| {
                        egui::Frame::none()
                            .fill(c.editor_bg)
                            .inner_margin(egui::Margin::ZERO)
                            .show(&mut cols[0], |ui| {
                                ui.set_height(ui.available_height());
                                self.draw_editor(ui);
                            });
                        egui::Frame::none()
                            .fill(c.preview_bg)
                            .inner_margin(egui::Margin::ZERO)
                            .show(&mut cols[1], |ui| {
                                ui.set_height(ui.available_height());
                                self.draw_preview(ui);
                            });
                    });
                }
                ViewMode::EditorOnly => {
                    egui::Frame::none().fill(c.editor_bg).show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        self.draw_editor(ui);
                    });
                }
                ViewMode::PreviewOnly => {
                    egui::Frame::none().fill(c.preview_bg).show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        self.draw_preview(ui);
                    });
                }
            });

        // Poll voice engine for results
        if let Some(ref eng) = self.voice_engine {
            while let Some(text) = eng.poll() {
                self.voice_terminal.push_str(&text);
                self.voice_terminal.push('\n');
                self.current_transcript = text;
            }
            ctx.request_repaint();
        }

        // Terminal test window for voice transcription
        if self.voice_engine.is_some() {
            let mut open = true;
            egui::Window::new("Terminal")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .default_height(300.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.label(&self.voice_terminal);
                        });
                });
            if !open {
                // Window closed -> stop voice
                if let Some(ref eng) = self.voice_engine {
                    eng.stop();
                }
                self.voice_engine = None;
            }
        }

        self.draw_find_bar(ctx);
        self.draw_settings_window(ctx);
        self.draw_quick_switcher(ctx);

        // Confirmation dialog for Empty Trash
        if self.confirm_empty_trash {
            let trash_count = self.notes.iter().filter(|n| n.trashed).count();
            egui::Window::new("Empty Trash")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Permanently delete {} trashed note{}? This cannot be undone.",
                        trash_count,
                        if trash_count == 1 { "" } else { "s" }
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Yes, delete all").clicked() {
                            self.empty_trash();
                            self.confirm_empty_trash = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_empty_trash = false;
                        }
                    });
                });
        }

        // Save on close
        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_notes();
            self.save_settings();
        }
    }
}
