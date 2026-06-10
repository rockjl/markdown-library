use crate::note::Note;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct WikiLink {
    pub target: String,
    pub alias: Option<String>,
}

/// Parse all `[[target]]` and `[[target|alias]]` wikilinks from the text.
pub fn extract(text: &str) -> Vec<WikiLink> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(end) = find_closing(&text[i + 2..]) {
                let inner = &text[i + 2..i + 2 + end];
                // Skip if contains newline (not a wikilink)
                if !inner.contains('\n') {
                    let (target, alias) = if let Some(pipe) = inner.find('|') {
                        (
                            inner[..pipe].trim().to_string(),
                            Some(inner[pipe + 1..].trim().to_string()),
                        )
                    } else {
                        (inner.trim().to_string(), None)
                    };
                    if !target.is_empty() {
                        out.push(WikiLink { target, alias });
                    }
                }
                i += 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn find_closing(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b']' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Find indices of notes whose title equals `target` (case-insensitive).
pub fn resolve<'a>(notes: &'a [Note], target: &str) -> Vec<usize> {
    let target_lc = target.to_lowercase();
    notes
        .iter()
        .enumerate()
        .filter(|(_, n)| !n.trashed && n.title.to_lowercase() == target_lc)
        .map(|(i, _)| i)
        .collect()
}

/// Build a map: note index → list of (source_note_index, link) that target it.
pub fn build_backlink_index(notes: &[Note]) -> HashMap<usize, Vec<(usize, WikiLink)>> {
    let mut index: HashMap<usize, Vec<(usize, WikiLink)>> = HashMap::new();
    for (src_idx, src_note) in notes.iter().enumerate() {
        if src_note.trashed {
            continue;
        }
        let links = extract(&src_note.content);
        for link in links {
            for tgt_idx in resolve(notes, &link.target) {
                if tgt_idx == src_idx {
                    continue;
                }
                index
                    .entry(tgt_idx)
                    .or_default()
                    .push((src_idx, link.clone()));
            }
        }
    }
    index
}

/// Preprocess markdown to make wikilinks visible (bold + brackets preserved)
/// so they stand out in the preview pane.
pub fn render_for_preview(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let bytes = markdown.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(end) = find_closing(&markdown[i + 2..]) {
                let inner = &markdown[i + 2..i + 2 + end];
                if !inner.contains('\n') {
                    let display = if let Some(pipe) = inner.find('|') {
                        &inner[pipe + 1..]
                    } else {
                        inner
                    };
                    out.push_str(&format!("**[{}]**", display.trim()));
                    i += 2 + end + 2;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    // Re-encode properly for non-ASCII
    if !markdown.is_ascii() {
        return render_for_preview_unicode(markdown);
    }
    out
}

fn render_for_preview_unicode(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut chars = markdown.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '[' {
            if let Some(&(_, '[')) = chars.peek() {
                let rest = &markdown[i + c.len_utf8()..];
                let after_brackets = &rest[1..]; // skip second '['
                if let Some(end) = find_closing(after_brackets) {
                    let inner = &after_brackets[..end];
                    if !inner.contains('\n') {
                        let display = if let Some(pipe) = inner.find('|') {
                            inner[pipe + 1..].trim()
                        } else {
                            inner.trim()
                        };
                        out.push_str(&format!("**[{}]**", display));
                        // Advance chars iterator past [[inner]]
                        let consumed = 1 + end + 2; // '[' already eaten + inner + "]]"
                        let target_byte = i + c.len_utf8() + consumed;
                        while let Some(&(b, _)) = chars.peek() {
                            if b >= target_byte {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    }
                }
            }
        }
        out.push(c);
    }
    out
}

#[derive(Default, Clone)]
pub struct QuickSwitcherState {
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub focus_query: bool,
}

impl QuickSwitcherState {
    pub fn open(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.focus_query = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

/// Fuzzy match score: lower is better. None if no match.
pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let h_lc = haystack.to_lowercase();
    let n_lc = needle.to_lowercase();
    // Substring match wins
    if let Some(pos) = h_lc.find(&n_lc) {
        return Some(pos as i32);
    }
    // Character-by-character subsequence match
    let mut h_iter = h_lc.chars();
    let mut score = 0i32;
    let mut last_pos = 0i32;
    for n_ch in n_lc.chars() {
        let mut found = false;
        let mut steps = 0i32;
        for h_ch in h_iter.by_ref() {
            steps += 1;
            if h_ch == n_ch {
                score += steps + last_pos.max(0);
                last_pos = steps;
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    Some(1000 + score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::Note;

    #[test]
    fn test_extract_basic() {
        let s = "This is a [[Target]] and [[Other|Alias]] text.";
        let links = extract(s);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "Target");
        assert_eq!(links[0].alias, None);
        assert_eq!(links[1].target, "Other");
        assert_eq!(links[1].alias.as_deref(), Some("Alias"));
    }

    #[test]
    fn test_resolve_and_backlinks() {
        let mut notes = Vec::new();
        notes.push(Note::new("A", "Content with [[B]]"));
        notes.push(Note::new("B", "Target note"));
        let res = resolve(&notes, "B");
        assert_eq!(res, vec![1usize]);
        let idx = build_backlink_index(&notes);
        assert!(idx.contains_key(&1));
        let backlinks = idx.get(&1).unwrap();
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].0, 0);
    }

    #[test]
    fn test_render_for_preview_ascii() {
        let md = "See [[Page|the page]] for more.";
        let out = render_for_preview(md);
        assert!(out.contains("**[the page]**"));
    }

    #[test]
    fn test_fuzzy_match() {
        assert_eq!(fuzzy_match("hello world", "world"), Some(6));
        assert!(fuzzy_match("abcd", "ad").is_some());
        assert!(fuzzy_match("abcd", "xz").is_none());
    }
}
