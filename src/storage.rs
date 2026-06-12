//! Persistent storage: notes, settings, search index, and question markers.

use crate::note::Note;
use crate::search::index::SearchIndex;
use crate::settings::Settings;
use std::fs;
use std::path::PathBuf;

/// Return the application data directory.
///
/// Uses `$APPDATA/markdown-library` on Windows, `.markdown-library` on Linux.
/// The directory is created if it does not exist.
pub fn data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("markdown-library");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    let p = PathBuf::from(".markdown-library");
    let _ = fs::create_dir_all(&p);
    p
}

fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn content_dir() -> PathBuf {
    data_dir().join("content")
}

fn index_file() -> PathBuf {
    data_dir().join("index.json")
}

/// Load all notes from the content directory.
///
/// Each `.md` file is parsed: if it contains YAML front matter the note metadata
/// is deserialised from it; otherwise the file's content is used as-is with a
/// default `Note` struct.  Returns an empty vec if the content directory is
/// missing or empty.
pub fn load_notes() -> Vec<Note> {
    let content = content_dir();
    if !content.exists() || !content.is_dir() {
        return Vec::new();
    }
    let mut notes = Vec::new();
    if let Ok(entries) = fs::read_dir(&content) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(s) = fs::read_to_string(&p) {
                    if let Some((meta, body)) = split_front_matter(&s) {
                        if let Ok(mut n) = serde_yaml::from_str::<Note>(&meta) {
                            n.content = body.to_string();
                            n.path = Some(p.clone());
                            notes.push(n);
                            continue;
                        }
                    }
                    let mut n = Note::default();
                    if let Some(fname) = p.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(id) = fname.parse::<u64>() {
                            n.id = id;
                        }
                    }
                    n.content = s;
                    n.path = Some(p.clone());
                    notes.push(n);
                }
            }
        }
    }
    notes
}

/// Load the pre-computed search index from disk.
///
/// Returns `None` if the index file does not exist or cannot be parsed.
pub fn load_index() -> Option<SearchIndex> {
    let path = index_file();
    if !path.exists() {
        return None;
    }
    fs::read_to_string(&path).ok().and_then(|s| SearchIndex::from_json(&s).ok())
}

/// Serialise the search index to disk as JSON.
pub fn save_index(index: &SearchIndex) {
    let path = index_file();
    if let Ok(json) = index.to_json() {
        let _ = fs::write(&path, json);
    }
}

/// Persist all notes to the content directory as `.md` files with YAML front matter.
///
/// Each note is written atomically via a `.md.tmp` temporary file followed by a rename.
pub fn save_notes(notes: &[Note]) {
    let content = content_dir();
    let _ = fs::create_dir_all(&content);

    for n in notes.iter() {
        let mut meta = n.clone();
        meta.content = String::new();
        meta.modified = false;
        if let Ok(yaml) = serde_yaml::to_string(&meta) {
            let body = &n.content;
            let md = format!("---\n{}---\n\n{}", yaml, body);
            let filename = content.join(format!("{}.md", n.id));
            let tmp = filename.with_extension("md.tmp");
            if fs::write(&tmp, md.as_bytes()).is_ok() {
                let _ = fs::rename(&tmp, &filename);
            } else {
                let _ = fs::write(&filename, md.as_bytes());
            }
        }
    }
}

fn split_front_matter(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    if !s.starts_with("---") {
        return None;
    }
    let mut sp = s.splitn(2, "\n---\n");
    if let Some(first) = sp.next() {
        if let Some(rest) = sp.next() {
            if let Some(meta) = first.strip_prefix("---\n") {
                let rest = if rest.starts_with('\n') { &rest[1..] } else { rest };
                return Some((meta.to_string(), rest));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::split_front_matter;

    #[test]
    fn test_split_front_matter_basic() {
        let input = "---\ntitle: Test\n---\n\nBody here";
        let res = split_front_matter(input).expect("should parse");
        assert!(res.0.contains("title: Test"));
        assert_eq!(res.1.trim(), "Body here");
    }

    #[test]
    fn test_split_front_matter_no_front() {
        let input = "No front matter\n# Title\nContent";
        assert!(split_front_matter(input).is_none());
    }

    #[test]
    fn test_split_front_matter_with_extra_newlines() {
        let input = "   ---\nkey: val\n---\n\nMore\nLines\n";
        let res = split_front_matter(input).expect("should parse even with leading spaces");
        assert!(res.0.contains("key: val"));
        assert!(res.1.contains("More"));
    }
}

/// Load application settings from `settings.json`.
///
/// Returns `Settings::default()` if the file is missing or corrupt.
pub fn load_settings() -> Settings {
    let path = settings_file();
    if !path.exists() {
        return Settings::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist application settings to `settings.json`.
pub fn save_settings(settings: &Settings) {
    let path = settings_file();
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}

fn search_history_file() -> PathBuf {
    data_dir().join("search_history.json")
}

/// Load saved search history from `search_history.json`.
///
/// Returns an empty vec if the file is missing or corrupt.
pub fn load_search_history() -> Vec<String> {
    let path = search_history_file();
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist search history to `search_history.json`.
pub fn save_search_history(history: &[String]) {
    let path = search_history_file();
    if let Ok(json) = serde_json::to_string_pretty(history) {
        let _ = fs::write(path, json);
    }
}

fn question_markers_file() -> PathBuf {
    data_dir().join("question_markers.json")
}

/// Built-in default question markers for voice search.
pub fn default_question_markers() -> Vec<String> {
    [
        "what is", "what are", "why", "how", "when", "where",
        "explain", "could you explain", "can you explain", "would you explain", "please explain",
        "describe", "can you describe", "could you describe", "would you describe",
        "tell me about", "could you tell me about", "can you tell me about",
        "walk me through", "could you walk me through", "can you walk me through",
        "why did you choose", "why did you pick", "why do you use", "why are you using",
        "what made you choose", "what makes you choose",
        "difference between", "what is the difference between",
        "compare", "compare with", "compare to", "versus", "vs",
        "how does", "how would you", "how do you", "how did you", "how have you",
        "what happens when", "what would happen if",
        "how would you handle", "how do you handle",
        "how would you design", "design a", "design an", "how would you implement",
        "have you ever", "can you share", "could you share", "tell me about a time",
        "in your project", "in your experience", "in your previous project",
        "how does solana", "how does anchor", "how does account", "how does pda work",
        "how does ownership", "how does borrowing", "how does lifetime", "how does trait",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

/// Load question markers from disk, falling back to built-in defaults.
pub fn load_question_markers() -> Vec<String> {
    let path = question_markers_file();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(default_question_markers)
}

/// Save question markers to disk.
pub fn save_question_markers(markers: &[String]) {
    let path = question_markers_file();
    if let Ok(json) = serde_json::to_string_pretty(markers) {
        let _ = fs::write(path, json);
    }
}
