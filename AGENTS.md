# AGENTS.md — markdown-library

## Entrypoint & build

- Single crate (not a workspace), binary at `src/main.rs` → `MarkdownApp` in `src/ui/app.rs`
- `edition = "2024"` — requires Rust **≥ 1.85** or nightly
- Dev: `cargo check` (fastest feedback) / `cargo build` / `cargo test` (31 unit tests)
- Release: `cargo build --release` (LTO enabled, single static binary)
- `build.rs` generates app icon (PNG + ICO); Windows embeds ICO via `winresource`
- CI: `.github/workflows/release.yml` — tag `v*` triggers Windows/Linux/macOS builds

## Data storage

- `storage::data_dir()` → `$APPDATA/markdown-library/` (Windows) or `.markdown-library/` (Linux)
- Notes: `<data_dir>/content/<id>.md` (YAML front-matter + body)
- Search index: `<data_dir>/index.json` (pre-computed tokens, rebuilt on every save)
- Settings: `<data_dir>/settings.json`
- Question markers: `<data_dir>/question_markers.json` — voice search trigger phrases loaded at runtime; if missing, falls back to built-in defaults in `storage::default_question_markers()`
- `/.markdown-library` is **gitignored** — copy `question_markers.json` template from `storage.rs` or create by running the app once

## Architecture

### Search — two paths, one module

Files in `src/search/`:
- `normalize.rs`, `synonym.rs` — stable token pipeline ⚠️ DO NOT EDIT
- `index.rs` — `SearchIndex` + `IndexedNote` (serialized to `index.json`) ⚠️ DO NOT EDIT
- `score.rs` — jaccard + lcs + note_score (title/tags/content weights) — open for tuning
- `matcher.rs` — `search(query, threshold) → Vec<SearchHit>`
- `transcript_processor.rs` — `extract_questions` + `split_queries` + `process_transcript`

Sidebar text search uses `matcher::search()`. ASR voice search uses `transcript_processor::process_transcript()` which loads question-pattern markers from `question_markers.json` at runtime.

### Voice / ASR

- `src/voice.rs` — `VoiceEngine` wrapping Xunfei WebSocket API
- `src/asr/mod.rs` — `TranscriptProvider` trait (future: Whisper, Azure)
- **F12** Push-To-Talk: press to start recording → press again to stop → `stop_and_search()` → `process_transcript()`
- Debug logs written to `voice_debug.log` (overwritten on each launch, gitignored)

### Library vs Interview mode

Sidebar toggle switches between Library (tag-grouped, opens edit) and Interview (semantic search, opens preview). `selected: Option<usize>` — when `None`, sidebar + editor both empty.

### UI module layout

UI code lives under `src/ui/`:
- `app.rs` — `MarkdownApp` struct, state, shortcuts, top-level layout (`SidePanel` + `CentralPanel`)
- `components/` — menu bar, status bar, toolbar, find bar
- `editor/` — source editor (`draw_editor`) and live preview (`draw_preview`)
- `panels/` — sidebar, table of contents, backlinks
- `windows/` — quick-switcher popup, settings window

### Debug logging

- `src/debug_log.rs` — `log_msg()` + `timestamp()` shared by `voice.rs` for operational ASR debug logging to `voice_debug.log`. Not a test-only utility.

### Performance

- `highlight::warmup()` called at startup forces syntect regex compilation (~572 ms) so the first editor render is fast. Without it, the first `layout_markdown()` call blocks for ~700 ms when library mode is toggled.

## Key shortcuts

| Key | Action |
|-----|--------|
| `F12` | Start/stop voice recording |
| `Ctrl+N` | New note |
| `Ctrl+S` | Save current note |
| `Ctrl+P` | Quick switcher |
| `Ctrl+F` / `Ctrl+H` | Find / Replace |
| `Ctrl+\` | Cycle view mode (editor / split / preview) |
| `Alt+↑/↓` | Move line |

## Testing

```sh
cargo test                       # all 31 unit tests
cargo test search::score         # single module
```

No integration tests, no external services. Tests do not touch disk.

## Constraints

- `plan.md` tracks pending/finished tasks — read it before starting new work
