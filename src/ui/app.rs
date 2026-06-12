// Copyright (c) 2025 markdown-library

use crate::attachments;
use crate::editor_actions::EditorAction;
use crate::export;
use crate::find_replace::FindReplaceState;
use crate::note::Note;
use crate::search::index::SearchIndex;
use crate::search::matcher::SearchHit;
use crate::settings::{FontChoice, Settings, ViewMode};
use crate::storage;
use crate::theme::self;
use crate::voice::VoiceEngine;
use crate::watcher::FSWatcher;
use crate::wikilinks::{self, QuickSwitcherState};

use egui_commonmark::CommonMarkCache;
use std::fs;
use std::time::Instant;

use super::constants::*;

/// Top-level application state for the Markdown Library.
pub struct MarkdownApp {
    /// All notes loaded from disk (trashed and active).
    pub(crate) notes: Vec<Note>,
    /// Index of the currently selected note (`None` if nothing is selected).
    pub(crate) selected: Option<usize>,
    /// When `true` the sidebar groups notes by tag; `false` for semantic-search mode.
    pub(crate) library_mode: bool,
    /// Cache used by the CommonMark preview renderer.
    pub(crate) cache: CommonMarkCache,
    /// Ring buffer of recent search queries.
    pub(crate) search_history: Vec<String>,
    /// Maximum number of search-history entries kept.
    pub(crate) max_history: usize,
    /// Tags the user has filtered the sidebar by.
    pub(crate) selected_tags: Vec<String>,
    /// Current sidebar search/text filter query.
    pub(crate) search_query: String,
    /// Number of sidebar items rendered so far (pagination).
    pub(crate) sidebar_loaded_count: usize,
    /// Number of sidebar items rendered per batch.
    pub(crate) sidebar_batch: usize,
    /// Distinguishes sidebar search from general search (used internally).
    pub(crate) sidebar_filter_key: String,
    /// Indices of notes that pass the current sidebar filter.
    pub(crate) sidebar_visible_indices: Vec<usize>,
    /// Whether the editing toolbar is in collapsed (single-row) mode.
    pub(crate) toolbar_collapsed: bool,
    /// When `true`, the sidebar search input will receive focus on the next frame.
    pub(crate) focus_sidebar_search: bool,
    /// A queued editor action to apply on the next frame (e.g. bold, italic).
    pub(crate) pending_action: Option<EditorAction>,
    /// A queued line-move action (`true` = up, `false` = down).
    pub(crate) pending_line_move: Option<bool>,
    /// Whether the editor should auto-continue a list on the next frame.
    pub(crate) pending_list_continuation: bool,
    /// User settings (theme, font, view mode, etc.).
    pub(crate) settings: Settings,
    /// Timestamp of the last manual or automatic save.
    last_save_at: Instant,
    /// Whether unsaved changes exist.
    pub(crate) notes_dirty: bool,
    /// Whether the settings window is open.
    pub(crate) show_settings: bool,
    /// Whether the trash view is active.
    pub(crate) show_trash: bool,
    /// State for the find-and-replace UI.
    pub(crate) find: FindReplaceState,
    /// State for the Ctrl+P quick-switcher popup.
    pub(crate) quick_switcher: QuickSwitcherState,
    /// Whether the backlinks panel is open.
    pub(crate) show_backlinks: bool,
    /// Index of the note whose backlinks we are currently viewing.
    pub(crate) current_backlinks_target: Option<usize>,
    /// Confirmation dialog for restoring default settings.
    pub(crate) confirm_restore_defaults: bool,
    /// Confirmation dialog for emptying the trash.
    pub(crate) confirm_empty_trash: bool,
    /// IDs of notes created but not yet saved (used to adjust view mode after first save).
    new_note_ids: std::collections::HashSet<u64>,
    /// Optional filesystem watcher monitoring the content directory.
    fs_watcher: Option<FSWatcher>,
    /// A temporary status-bar message override.
    pub(crate) status_override: Option<String>,
    /// When the status-bar override was set (for auto-dismiss).
    pub(crate) status_override_at: Option<Instant>,
    /// Optional ASR voice engine instance.
    pub(crate) voice_engine: Option<VoiceEngine>,
    /// Terminal output from voice-engine debug logs.
    voice_terminal: String,
    /// Results from the most recent voice search.
    pub(crate) voice_search_results: Vec<SearchHit>,
    /// When `true` the preview is shown for the voice-search top result.
    voice_preview_mode: bool,
    /// The raw transcript from the most recent voice recording.
    pub(crate) current_transcript: String,
    /// Pre-computed search index for full-text search.
    pub(crate) search_index: Option<SearchIndex>,
    /// Single tag filter selected via popup; `None` means show all.
    pub(crate) selected_tag: Option<String>,
    /// Stack of (note_index, unix_timestamp) for recently viewed notes, newest first.
    pub(crate) note_history: Vec<(usize, i64)>,
    /// When true, show recent-history view instead of full list.
    pub(crate) show_recent: bool,
    /// When true, the tag selection popup is open.
    pub(crate) show_tag_popup: bool,
}

impl MarkdownApp {
    /// Create a new application instance, loading notes, settings, and search index from disk.
    ///
    /// * `cc` — egui/eframe creation context used to load fonts and apply the initial theme.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let settings = storage::load_settings();

        load_user_font(&cc.egui_ctx, settings.font_choice);

        theme::apply(&cc.egui_ctx, settings.theme, settings.editor_font_size);

        let notes = storage::load_notes();

        let search_index = Some(storage::load_index()
            .unwrap_or_else(|| SearchIndex::build(&notes)));

        crate::highlight::warmup();

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
            selected_tag: None,
            note_history: Vec::new(),
            show_recent: false,
            show_tag_popup: false,
        };
        app.settings.view_mode = ViewMode::PreviewOnly;
        app
    }

    /// Persist all notes to disk and rebuild the search index.
    pub(crate) fn save_notes(&mut self) {
        storage::save_notes(&self.notes);
        let content_dir = storage::content_dir();
        for n in self.notes.iter_mut() {
            if n.path.is_none() {
                n.path = Some(content_dir.join(format!("{}.md", n.id)));
            }
            n.modified = false;
        }
        self.search_index = Some(SearchIndex::build(&self.notes));
        if let Some(index) = &self.search_index {
            storage::save_index(index);
        }
        self.notes_dirty = false;
        self.last_save_at = Instant::now();
    }

    /// Stop the voice engine, poll the transcript, and run it through `process_transcript`.
    ///
    /// Updates `voice_search_results`, `voice_preview_mode`, and auto-selects the best hit.
    pub(crate) fn stop_and_search(&mut self) {
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

    /// Save the current settings (including collapsed state) to disk.
    pub(crate) fn save_settings(&self) {
        let mut s = self.settings.clone();
        s.toolbar_collapsed = self.toolbar_collapsed;
        storage::save_settings(&s);
    }

    /// Mark the notes as having unsaved changes.
    pub(crate) fn mark_dirty(&mut self) {
        self.notes_dirty = true;
    }

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
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
        let key_v = ctx.input(|i| i.key_pressed(egui::Key::V));
        let paste_event = ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Paste(_)))
        });
        if paste_event || (ctrl && !shift && key_v) {
            self.try_paste_image(false);
        }
        if ctrl && shift && key_v {
            self.try_paste_image(true);
        }
        let key_p = ctx.input(|i| i.key_pressed(egui::Key::P));
        if ctrl && key_p && !shift {
            self.quick_switcher.open();
        }
        if key_esc && self.quick_switcher.visible {
            self.quick_switcher.close();
        }
        let key_comma = ctx.input(|i| i.key_pressed(egui::Key::Comma));
        if ctrl && key_comma {
            self.settings.view_mode = ViewMode::EditorOnly;
            self.save_settings();
        }
        let key_period = ctx.input(|i| i.key_pressed(egui::Key::Period));
        if ctrl && key_period {
            self.settings.view_mode = ViewMode::PreviewOnly;
            self.save_settings();
        }
        let key_slash = ctx.input(|i| i.key_pressed(egui::Key::Slash));
        if ctrl && key_slash {
            self.focus_sidebar_search = true;
        }
        let key_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
        if ctrl && key_down && !shift {
            self.select_first_filtered_note();
        }
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

    pub(crate) fn select_first_filtered_note(&mut self) {
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

    /// Draw a single toolbar button that queues `action` when clicked.
    /// Export the selected note to a location chosen via a native save dialog.
    pub(crate) fn save_current_to_file(&mut self) {
        self.save_notes();
        for n in self.notes.iter_mut() {
            n.modified = false;
        }
    }

    /// Open a Markdown file via a native file dialog and import it as a new note.
    pub(crate) fn open_file(&mut self) {
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
            note.path = Some(path.clone());
            self.notes.push(note);
            self.selected = Some(self.notes.len() - 1);
            self.settings.view_mode = ViewMode::PreviewOnly;
            self.mark_dirty();
        }
    }

    /// Create a new blank note, select it, and open the editor.
    pub(crate) fn new_note(&mut self) {
        self.notes.push(Note::default());
        let idx = self.notes.len() - 1;
        self.selected = Some(idx);
        self.new_note_ids.insert(self.notes[idx].id);
        self.settings.view_mode = ViewMode::EditorOnly;
        self.mark_dirty();
    }

    /// Move the note at `idx` to the trash (soft-delete).
    pub(crate) fn move_to_trash(&mut self, idx: usize) {
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

    /// Restore a trashed note back to active status.
    pub(crate) fn restore_from_trash(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.notes[idx].trashed = false;
        self.notes[idx].touch();
        self.mark_dirty();
    }

    /// Permanently delete the note at `idx` from disk and memory.
    pub(crate) fn delete_permanently(&mut self, idx: usize) {
        if idx >= self.notes.len() {
            return;
        }
        self.new_note_ids.remove(&self.notes[idx].id);
        let content_dir = storage::content_dir();
        let path = content_dir.join(format!("{}.md", self.notes[idx].id));
        let _ = std::fs::remove_file(&path);
        self.notes.remove(idx);
        if self.selected.map_or(false, |s| s >= self.notes.len()) {
            self.selected = Some(self.notes.len().saturating_sub(1));
        }
        self.mark_dirty();
    }

    /// Permanently delete every trashed note.
    pub(crate) fn empty_trash(&mut self) {
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

    /// Ensure the currently selected index points to a valid note, or set it to `None`.
    pub(crate) fn ensure_valid_selection(&mut self) {
        let Some(sel) = self.selected else {
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
                if !self.show_trash {
                    self.notes.push(Note::default());
                    self.selected = Some(self.notes.len() - 1);
                    self.mark_dirty();
                }
            }
        }
    }

    /// Initialise the filesystem watcher on the content directory if not already running.
    pub(crate) fn ensure_fs_watcher(&mut self) {
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


    /// Draw the markdown editor panel for the selected note.







    /// Select the note at `idx` and switch to preview mode if it has a file on disk.
    pub(crate) fn select_note_by_index(&mut self, idx: usize) {
        if idx < self.notes.len() {
            self.selected = Some(idx);
        }
    }

    /// Navigate to the first note whose title matches `target` (case-insensitive).
    pub(crate) fn navigate_to_wikilink(&mut self, target: &str) {
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




    /// Attempt to paste an image from the clipboard into the selected note.
    ///
    /// When `force_message` is true, always show an informational status message.
    pub(crate) fn try_paste_image(&mut self, force_message: bool) {
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

    /// Export the currently selected note as HTML and copy it to the clipboard.
    pub(crate) fn export_html(&self) {
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


    /// Automatically save notes if `AUTOSAVE_INTERVAL_SECS` has elapsed since the last save.
    pub(crate) fn auto_save_if_needed(&mut self) {
        if self.settings.auto_save
            && self.notes_dirty
            && self.last_save_at.elapsed().as_secs() >= AUTOSAVE_INTERVAL_SECS
        {
            self.save_notes();
        }
    }
}

/// Load the chosen font into egui's font system.
///
/// Falls back through candidate paths then common OS font locations.
pub(crate) fn load_user_font(ctx: &egui::Context, font_choice: FontChoice) {
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

    let common_fallbacks: &[&str] = &[
        r"C:\Windows\Fonts\SegoeUI.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Arial.ttf",
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
    /// Called every frame by egui to render the full application UI.
    ///
    /// This is the main frame-entry point: it handles shortcuts, loads external changes
    /// via the filesystem watcher, auto-saves, and draws the sidebar, editor, preview,
    /// settings, find/replace, backlinks, quick-switcher, and voice-search panels.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        self.handle_shortcuts(ctx);
        self.ensure_valid_selection();
        self.ensure_fs_watcher();
        if let Some(w) = &self.fs_watcher {
            while let Some(ev_res) = w.try_recv() {
                match ev_res {
                    Ok(_ev) => {
                        if self.last_save_at.elapsed().as_millis() < 1500 {
                            continue;
                        }
                        if !self.notes_dirty && self.notes.iter().all(|n| !n.modified) {
                            let old_selected_title = self.selected.and_then(|s| self.notes.get(s)).map(|n| n.title.clone());
                            let new_notes = storage::load_notes();
                            if !new_notes.is_empty() {
                                self.notes = new_notes;
                                if let Some(idx) = storage::load_index() {
                                    self.search_index = Some(idx);
                                } else {
                                    self.search_index = Some(SearchIndex::build(&self.notes));
                                }
                                if let Some(title) = old_selected_title {
                                    if let Some(idx) = self.notes.iter().position(|n| n.title == title) {
                                        self.selected = Some(idx);
                                    }
                                }
                                self.status_override = Some("Notes reloaded due to external change".to_string());
                                self.status_override_at = Some(Instant::now());
                            }
                        } else {
                            self.status_override = Some("External changes detected; not reloaded due to unsaved edits".to_string());
                            self.status_override_at = Some(Instant::now());
                        }
                    }
                    Err(_) => {
                        self.status_override = Some("Filesystem watch error".to_string());
                        self.status_override_at = Some(Instant::now());
                    }
                }
            }
        }
        self.auto_save_if_needed();

        self.draw_menu_bar(ctx);

        self.draw_status_bar(ctx);

        let c = self.colors();

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

        if let Some(ref eng) = self.voice_engine {
            while let Some(text) = eng.poll() {
                self.voice_terminal.push_str(&text);
                self.voice_terminal.push('\n');
                self.current_transcript = text;
            }
            ctx.request_repaint();
        }

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
                if let Some(ref eng) = self.voice_engine {
                    eng.stop();
                }
                self.voice_engine = None;
            }
        }

        self.draw_find_bar(ctx);
        self.draw_settings_window(ctx);
        self.draw_quick_switcher(ctx);

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

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_notes();
            self.save_settings();
        }
    }
}
