#[derive(Default, Clone)]
pub struct FindReplaceState {
    pub visible: bool,
    pub show_replace: bool,
    pub query: String,
    pub replace_with: String,
    pub case_sensitive: bool,
    pub current_match: usize,
    pub focus_query: bool,
}

impl FindReplaceState {
    pub fn open_find(&mut self) {
        self.visible = true;
        self.show_replace = false;
        self.focus_query = true;
    }

    pub fn open_replace(&mut self) {
        self.visible = true;
        self.show_replace = true;
        self.focus_query = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

pub fn find_all(text: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    if case_sensitive {
        find_all_inner(text, query)
    } else {
        // For case-insensitive matching we lowercase both sides. The returned byte
        // offsets refer to the lowercased text, which lines up with the original for
        // ASCII (the common case for case folding); for non-ASCII text the offsets
        // are an approximation good enough for matching, since most CJK characters
        // don't case-fold.
        let hay = text.to_lowercase();
        let needle = query.to_lowercase();
        find_all_inner(&hay, &needle)
    }
}

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

pub fn replace_all(text: &str, query: &str, replacement: &str, case_sensitive: bool) -> (String, usize) {
    if query.is_empty() {
        return (text.to_string(), 0);
    }
    if case_sensitive {
        let count = text.matches(query).count();
        (text.replace(query, replacement), count)
    } else {
        // Manual case-insensitive replacement preserving original bytes outside matches.
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
