use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub modified: bool,
    #[serde(default)]
    pub title_synced: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub starred: bool,
    #[serde(default)]
    pub trashed: bool,
    #[serde(default = "now_ts")]
    pub updated_at: u64,
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let ts = now_ts();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    ts.wrapping_mul(1_000_000) + counter
}

impl Note {
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: next_id(),
            title: title.into(),
            content: content.into(),
            path: None,
            modified: false,
            tags: Vec::new(),
            starred: false,
            trashed: false,
            updated_at: now_ts(),
            title_synced: true,
        }
    }

    pub fn display_title(&self) -> String {
        if self.modified {
            format!("{}*", self.title)
        } else {
            self.title.clone()
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = now_ts();
    }
}

impl Default for Note {
    fn default() -> Self {
        Self::new("Untitled", "# Untitled\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_default_and_new() {
        let n = Note::default();
        assert_eq!(n.title, "Untitled");
        assert!(n.content.starts_with("# Untitled"));
        let m = Note::new("T", "# T\n\n");
        assert_eq!(m.title, "T");
    }

    #[test]
    fn test_display_title_modified() {
        let mut n = Note::new("Hi", "# Hi\n\n");
        assert_eq!(n.display_title(), "Hi");
        n.modified = true;
        assert_eq!(n.display_title(), "Hi*");
    }

    #[test]
    fn test_touch_updates_timestamp() {
        let mut n = Note::new("X", "# X\n\n");
        let before = n.updated_at;
        sleep(Duration::from_millis(1100));
        n.touch();
        assert!(n.updated_at >= before);
    }
}
