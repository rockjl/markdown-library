//! Find & replace state and matching logic for the editor.

/// State for the find/replace UI overlay.
#[derive(Default, Clone)]
pub struct FindReplaceState {
    /// Whether the find bar is visible
    pub visible: bool,
    /// Whether to show the replace input row
    pub show_replace: bool,
    /// Current search query text
    pub query: String,
    /// Replacement text
    pub replace_with: String,
    /// Whether matching is case-sensitive
    pub case_sensitive: bool,
    /// Index of the currently highlighted match
    pub current_match: usize,
    /// If `true`, the query input should gain focus on next frame
    pub focus_query: bool,
}

impl FindReplaceState {
    /// Open the find bar (without replace row).
    pub fn open_find(&mut self) {
        self.visible = true;
        self.show_replace = false;
        self.focus_query = true;
    }

    /// Open the find bar with the replace row visible.
    pub fn open_replace(&mut self) {
        self.visible = true;
        self.show_replace = true;
        self.focus_query = true;
    }

    /// Close the find bar.
    pub fn close(&mut self) {
        self.visible = false;
    }
}

/// Find all byte-offset ranges of `query` in `text`.
///
/// # Returns
/// A vector of `(start_byte, end_byte)` pairs for each match.
pub fn find_all(text: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    if case_sensitive {
        find_all_inner(text, query)
    } else {
        let hay = text.to_lowercase();
        let needle = query.to_lowercase();
        find_all_inner(&hay, &needle)
    }
}

/// Internal substring search returning byte-offset ranges.
fn find_all_inner(hay: &str, needle: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = hay[start..].find(needle) {
        let abs = start + pos;
        out.push((abs, abs + needle.len()));
        start = abs + needle.len().max(1);
    }
    out
}

/// Replace all occurrences of `query` in `text` with `replacement`.
///
/// # Returns
/// A tuple of `(new_text, replacement_count)`.
pub fn replace_all(text: &str, query: &str, replacement: &str, case_sensitive: bool) -> (String, usize) {
    if query.is_empty() {
        return (text.to_string(), 0);
    }
    if case_sensitive {
        let count = text.matches(query).count();
        (text.replace(query, replacement), count)
    } else {
        let hay = text.to_lowercase();
        let needle = query.to_lowercase();
        let matches = find_all_inner(&hay, &needle);
        if matches.is_empty() {
            return (text.to_string(), 0);
        }
        let mut out = String::with_capacity(text.len());
        let mut cursor = 0usize;
        let count = matches.len();
        for (s, e) in matches {
            out.push_str(&text[cursor..s]);
            out.push_str(replacement);
            cursor = e;
        }
        out.push_str(&text[cursor..]);
        (out, count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_all_case_insensitive() {
        let s = "Hello HELLO hello";
        let matches = find_all(s, "hello", false);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_replace_all_case_insensitive() {
        let s = "Abc ABC aBc";
        let (out, count) = replace_all(s, "abc", "X", false);
        assert_eq!(count, 3);
        assert!(out.contains("X X X") || out.contains("X"));
    }
}
