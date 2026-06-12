pub enum NavTarget {
    Wiki(String),
    Index(usize),
}

pub struct ListMarkerInfo {
    pub indent: String,
    pub next_marker: String,
    pub content_empty: bool,
}

pub fn detect_list_marker(line: &str) -> Option<ListMarkerInfo> {
    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let after = &line[indent.len()..];

    if let Some(rest) = after.strip_prefix("- [ ] ").or_else(|| after.strip_prefix("- [x] ")).or_else(|| after.strip_prefix("- [X] ")) {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "- [ ] ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("- ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "- ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("* ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "* ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("+ ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "+ ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    if let Some(rest) = after.strip_prefix("> ") {
        return Some(ListMarkerInfo {
            indent,
            next_marker: "> ".to_string(),
            content_empty: rest.trim().is_empty(),
        });
    }
    let digit_end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 {
        let after_digits = &after[digit_end..];
        if let Some(rest) = after_digits.strip_prefix(". ") {
            let n: u32 = after[..digit_end].parse().unwrap_or(0);
            return Some(ListMarkerInfo {
                indent,
                next_marker: format!("{}. ", n + 1),
                content_empty: rest.trim().is_empty(),
            });
        }
    }
    None
}
