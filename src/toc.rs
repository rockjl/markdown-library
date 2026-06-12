//! Table-of-contents extraction from Markdown headings.

/// A single heading entry extracted from markdown.
#[derive(Clone, Debug)]
pub struct Heading {
    /// Heading level (1–6, where 1 is the highest).
    pub level: u8,
    /// Plain text of the heading (without `#` markers).
    pub text: String,
    /// Character offset of this heading in the original markdown string.
    pub char_offset: usize,
}

/// Extract all ATX-style (`#`-prefixed) headings from markdown text.
///
/// Returns a `Vec<Heading>` sorted in document order.
pub fn extract(markdown: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut char_offset = 0usize;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let leading_ws = line.chars().take_while(|c| c.is_whitespace() && *c != '\n').count();
        if let Some(level) = atx_level(trimmed) {
            let text = trimmed
                .trim_start_matches('#')
                .trim_start()
                .trim_end()
                .trim_end_matches('#')
                .trim_end()
                .to_string();
            headings.push(Heading {
                level,
                text,
                char_offset: char_offset + leading_ws,
            });
        }
        char_offset += line.chars().count();
    }
    headings
}

fn atx_level(line: &str) -> Option<u8> {
    let mut count = 0u8;
    for ch in line.chars() {
        if ch == '#' {
            count += 1;
            if count > 6 {
                return None;
            }
        } else if ch == ' ' && count > 0 {
            return Some(count);
        } else {
            return None;
        }
    }
    None
}
