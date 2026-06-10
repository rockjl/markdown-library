# Markdown Library

A lightweight, native Markdown + interview-prep editor built with Rust + egui.
Three-pane layout with semantic search, voice-powered ASR question retrieval (Push-To-Talk F12),
and a Library/Interview dual-mode sidebar.

## Features

- **Three-pane layout**: sidebar note list, source editor, live preview
- **Dual sidebar modes**: Library (tag-grouped browsing, edit on click) and Interview (semantic search first, preview on click)
- **Voice search (F12)**: Push-To-Talk recording via Xunfei ASR → `extract_questions()` → semantic search → ranked results in sidebar
- **Sidebar smart search**: prefix/substring match (single word) or semantic jaccard+LCS (multi-word queries)
- **Persistent search index**: `index.json` stores pre-computed tokens; rebuilt on every save — fast startup
- **Light / Dark theme** with persistent settings
- **Markdown formatting toolbar** with keyboard shortcuts (`Ctrl+B`, `Ctrl+I`, `Ctrl+E`, `Ctrl+K`, `Ctrl+Shift+L/T/Q`)
- **Find & Replace** (`Ctrl+F` / `Ctrl+H`) with case sensitivity toggle
- **Syntax highlighting** in the editor via `syntect`
- **Auto list continuation** for `-`, `*`, `+`, `>`, `- [ ]`, and numbered lists
- **Move lines** with `Alt+↑/↓`
- **Auto-save** to `.markdown-library/content/<id>.md`
- **Trash** with restore / permanent delete
- **Favorites** (★) and tags
- **TOC panel** with click-to-jump
- **Wikilinks** `[[Note]]` and `[[Note|Alias]]`
- **Backlinks panel** showing both incoming and outgoing links
- **Quick Switcher** (`Ctrl+P`) with fuzzy matching
- **View modes**: editor-only / split / preview-only (`Ctrl+\` to cycle)
- **HTML export**
- **Paste images** from clipboard (`Ctrl+V`) — saved to attachments folder

## Build

```sh
cargo build --release
```

The single executable lands at `target/release/markdown-library` (~8 MB, no external DLLs required on Windows).

### Linux

```sh
# System deps for egui/gtk
sudo apt install libgtk-3-dev libssl-dev
cargo build --release
```

### macOS

```sh
cargo build --release
```

## Data location

All data lives in `.markdown-library/` (local to the working directory, or `$APPDATA/markdown-library/` on Windows):

```
.markdown-library/
├── content/           # Notes as <id>.md with YAML front-matter (authoritative store)
├── index.json         # Pre-computed search tokens (built on save)
├── settings.json      # UI theme, font, sidebar batch
├── search_history.json
└── attachments/       # Pasted images
```

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `F12` | Push-To-Talk: start / stop voice recording → search |
| `Ctrl+N` | New note |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save to file |
| `Ctrl+F` | Find |
| `Ctrl+H` | Replace |
| `Ctrl+P` | Quick Switcher |
| `Ctrl+\` | Cycle view mode (edit / split / preview) |
| `Ctrl+B/I/E/K` | Bold / Italic / Inline code / Link |
| `Ctrl+Shift+L/T/Q` | Bullet / Todo / Quote |
| `Alt+↑/↓` | Move line up/down |
| `Ctrl+V` (with image) | Paste image as attachment |
| `Ctrl+/` | Focus sidebar search |

## Architecture overview

```
src/
├── main.rs                  — Binary entrypoint
├── app.rs                   — MarkdownApp (UI state, shortcuts, draw)
├── asr/mod.rs               — TranscriptProvider trait
├── voice.rs                 — VoiceEngine (Xunfei WebSocket)
├── search/
│   ├── index.rs             — SearchIndex + IndexedNote (persisted to index.json)
│   ├── normalize.rs         — Token normalization pipeline
│   ├── synonym.rs           — Synonym map (tell→explain, …)
│   ├── score.rs             — jaccard / LCS / note_score (title 60% + tags 35% + content 5%)
│   ├── matcher.rs           — search() with threshold
│   └── transcript_processor.rs — extract_questions + split_queries + process_transcript()
├── storage.rs               — Load/save notes, index, settings
├── note.rs                  — Note model
├── settings.rs              — Settings + ThemeMode + ViewMode
├── watcher.rs               — Filesystem watcher for content/*.md
├── editor_actions.rs        — Text mutation helpers
├── find_replace.rs          — Find/replace state
├── highlight.rs             — Search highlight
├── theme.rs                 — ThemeColors
├── toc.rs                   — Table of contents
├── wikilinks.rs             — [[Note]] resolver
└── attachments.rs           — Image paste handler
```

## License

MIT
