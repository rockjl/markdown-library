# Markdown Editor

A lightweight, native Markdown editor for Windows built with Rust + egui.
Obsidian-inspired note-taking with wikilinks, backlinks, and a quick switcher.

## Features

- **Three-pane layout**: sidebar note list, source editor, live preview
- **Light / Dark theme** with persistent settings
- **Markdown formatting toolbar** with keyboard shortcuts
  (`Ctrl+B`, `Ctrl+I`, `Ctrl+E`, `Ctrl+K`, `Ctrl+Shift+L/T/Q`)
- **Find & Replace** (`Ctrl+F` / `Ctrl+H`) with case sensitivity toggle
- **Syntax highlighting** in the editor via `syntect`
- **Auto list continuation** for `-`, `*`, `+`, `>`, `- [ ]`, and numbered lists
- **Move lines** with `Alt+↑/↓`
- **Auto-save** to `%APPDATA%\markdown-editor\notes.json`
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

The single executable lands at `target/release/markdown-editor.exe`
(~8 MB, no external DLLs required).

## Data location

- Notes: `%APPDATA%\markdown-editor\notes.json`
- Settings: `%APPDATA%\markdown-editor\settings.json`
- Attachments: `%APPDATA%\markdown-editor\attachments\`

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New note |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save to file |
| `Ctrl+F` | Find |
| `Ctrl+H` | Replace |
| `Ctrl+P` | Quick Switcher |
| `Ctrl+\` | Cycle view mode |
| `Ctrl+B/I/E/K` | Bold / Italic / Inline code / Link |
| `Ctrl+Shift+L/T/Q` | Bullet / Todo / Quote |
| `Alt+↑/↓` | Move line up/down |
| `Ctrl+V` (with image) | Paste image as attachment |

## License

MIT
