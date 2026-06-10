use crate::note::Note;
use crate::search::index::SearchIndex;
use crate::settings::Settings;
use std::fs;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("markdown-editor");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    let p = PathBuf::from(".markdown-editor");
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
                    // Fallback: name-based id
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

pub fn load_index() -> Option<SearchIndex> {
    let path = index_file();
    if !path.exists() {
        return None;
    }
    fs::read_to_string(&path).ok().and_then(|s| SearchIndex::from_json(&s).ok())
}

pub fn save_index(index: &SearchIndex) {
    let path = index_file();
    if let Ok(json) = index.to_json() {
        let _ = fs::write(&path, json);
    }
}

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
    // Split on the terminating marker "\n---\n"
    let mut sp = s.splitn(2, "\n---\n");
    if let Some(first) = sp.next() {
        if let Some(rest) = sp.next() {
            if let Some(meta) = first.strip_prefix("---\n") {
                // If the saved file used an extra blank line between the closing
                // front-matter marker and the body (common), strip a single
                // leading '\n' so we don't accumulate blank lines on repeated
                // load/save cycles.
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

pub fn save_settings(settings: &Settings) {
    let path = settings_file();
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}

fn search_history_file() -> PathBuf {
    data_dir().join("search_history.json")
}

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

pub fn save_search_history(history: &[String]) {
    let path = search_history_file();
    if let Ok(json) = serde_json::to_string_pretty(history) {
        let _ = fs::write(path, json);
    }
}


