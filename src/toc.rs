#[derive(Clone, Debug)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// Character index in the source where this heading line starts.
    pub char_offset: usize,
}

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
