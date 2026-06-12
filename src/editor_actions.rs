//! Markdown formatting actions applied to the text buffer.

/// Supported markdown formatting actions.
#[derive(Clone, Copy, Debug)]
pub enum EditorAction {
    /// Wrap the selection with a prefix and suffix (e.g. `**bold**`). Toggles if already wrapped.
    Wrap {
        /// Prefix string (e.g. `**`)
        prefix: &'static str,
        /// Suffix string (e.g. `**`)
        suffix: &'static str,
    },
    /// Prepend a string to each selected line (e.g. `- `, `> `, `# `). Toggles if already present.
    LinePrefix(&'static str),
    /// Insert raw text at the cursor, replacing the current selection.
    Insert(&'static str),
    /// Insert a fenced code block with an optional language identifier.
    CodeBlock(&'static str),
    /// Insert an empty table skeleton at the cursor.
    Table {
        /// Number of body rows
        rows: usize,
        /// Number of columns
        cols: usize,
    },
}

/// Result of applying an editor action.
pub struct ActionResult {
    /// The updated note content
    pub new_content: String,
    /// New cursor position (character index) after the action
    pub new_cursor_start: usize,
    /// New selection end (character index) after the action
    pub new_cursor_end: usize,
}

/// Apply an editor action to the given content and selection.
///
/// # Parameters
/// * `action` - The formatting action to apply
/// * `content` - The current note text
/// * `sel_start_char` - Selection start in character indices
/// * `sel_end_char` - Selection end in character indices
///
/// # Returns
/// An `ActionResult` with the modified content and updated cursor positions.
pub fn apply(
    action: EditorAction,
    content: &str,
    sel_start_char: usize,
    sel_end_char: usize,
) -> ActionResult {
    let (start, end) = if sel_start_char <= sel_end_char {
        (sel_start_char, sel_end_char)
    } else {
        (sel_end_char, sel_start_char)
    };

    match action {
        EditorAction::Wrap { prefix, suffix } => wrap_selection(content, start, end, prefix, suffix),
        EditorAction::LinePrefix(prefix) => line_prefix(content, start, end, prefix),
        EditorAction::Insert(text) => insert_text(content, start, end, text),
        EditorAction::CodeBlock(lang) => {
            let block = format!("\n```{}\n", lang);
            let after = "\n```\n";
            wrap_with_blocks(content, start, end, &block, after)
        }
        EditorAction::Table { rows, cols } => insert_table(content, start, end, rows, cols),
    }
}

/// Convert a character index to a byte index in the given string.
fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// Count characters in a string (Unicode-aware).
fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Wrap the selected text with prefix/suffix, toggling off if already wrapped.
fn wrap_selection(
    content: &str,
    start: usize,
    end: usize,
    prefix: &'static str,
    suffix: &'static str,
) -> ActionResult {
    let b_start = char_to_byte(content, start);
    let b_end = char_to_byte(content, end);
    let selected = &content[b_start..b_end];
    let before = &content[..b_start];
    let after = &content[b_end..];

    let p_chars = char_count(prefix);
    let s_chars = char_count(suffix);

    let already_wrapped_inside = selected.starts_with(prefix) && selected.ends_with(suffix)
        && char_count(selected) >= p_chars + s_chars;
    let already_wrapped_outside = before.ends_with(prefix) && after.starts_with(suffix);

    if already_wrapped_inside {
        let inner_byte_len = selected.len() - prefix.len() - suffix.len();
        let inner = &selected[prefix.len()..prefix.len() + inner_byte_len];
        let new_content = format!("{}{}{}", before, inner, after);
        return ActionResult {
            new_content,
            new_cursor_start: start,
            new_cursor_end: start + char_count(inner),
        };
    }
    if already_wrapped_outside {
        let new_before = &before[..before.len() - prefix.len()];
        let new_after = &after[suffix.len()..];
        let new_content = format!("{}{}{}", new_before, selected, new_after);
        return ActionResult {
            new_content,
            new_cursor_start: start - p_chars,
            new_cursor_end: end - p_chars,
        };
    }

    let new_content = format!("{}{}{}{}{}", before, prefix, selected, suffix, after);
    let new_start = start + p_chars;
    let new_end = end + p_chars;
    ActionResult {
        new_content,
        new_cursor_start: new_start,
        new_cursor_end: new_end,
    }
}

/// Prepend a prefix string to each line in the selection, toggling off if all lines already have it.
fn line_prefix(
    content: &str,
    start: usize,
    end: usize,
    prefix: &'static str,
) -> ActionResult {
    let b_start = char_to_byte(content, start);
    let b_end = char_to_byte(content, end);

    let line_start = content[..b_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = content[b_end..]
        .find('\n')
        .map(|i| b_end + i)
        .unwrap_or(content.len());

    let before = &content[..line_start];
    let region = &content[line_start..line_end];
    let after = &content[line_end..];

    let all_have = region
        .split('\n')
        .all(|l| l.starts_with(prefix) || l.is_empty());

    let p_chars = char_count(prefix);
    let mut new_region = String::new();
    let mut removed_first_line = 0usize;
    let mut added_total: i64 = 0;

    for (i, line) in region.split('\n').enumerate() {
        if i > 0 {
            new_region.push('\n');
        }
        if all_have {
            if line.starts_with(prefix) {
                new_region.push_str(&line[prefix.len()..]);
                if i == 0 {
                    removed_first_line = p_chars;
                }
                added_total -= p_chars as i64;
            } else {
                new_region.push_str(line);
            }
        } else {
            if prefix == "1. " {
                new_region.push_str(&format!("{}. {}", i + 1, line));
                let added = char_count(&format!("{}. ", i + 1));
                if i == 0 {
                    removed_first_line = 0;
                }
                added_total += added as i64;
                continue;
            }
            new_region.push_str(prefix);
            new_region.push_str(line);
            added_total += p_chars as i64;
            if i == 0 {
                removed_first_line = 0;
            }
        }
    }

    let new_content = format!("{}{}{}", before, new_region, after);
    let line_start_chars = char_count(&content[..line_start]);
    let first_line_shift: i64 = if all_have {
        -(removed_first_line as i64)
    } else if prefix == "1. " {
        char_count("1. ") as i64
    } else {
        p_chars as i64
    };
    let new_start = (start as i64 + first_line_shift).max(line_start_chars as i64) as usize;
    let new_end = (end as i64 + added_total).max(new_start as i64) as usize;

    ActionResult {
        new_content,
        new_cursor_start: new_start,
        new_cursor_end: new_end,
    }
}

/// Insert text at the cursor, replacing the current selection.
fn insert_text(content: &str, start: usize, end: usize, text: &str) -> ActionResult {
    let b_start = char_to_byte(content, start);
    let b_end = char_to_byte(content, end);
    let before = &content[..b_start];
    let after = &content[b_end..];
    let new_content = format!("{}{}{}", before, text, after);
    let new_pos = start + char_count(text);
    ActionResult {
        new_content,
        new_cursor_start: new_pos,
        new_cursor_end: new_pos,
    }
}

/// Wrap the selection with block-level open/close strings (e.g. code fences).
fn wrap_with_blocks(
    content: &str,
    start: usize,
    end: usize,
    block_open: &str,
    block_close: &str,
) -> ActionResult {
    let b_start = char_to_byte(content, start);
    let b_end = char_to_byte(content, end);
    let selected = &content[b_start..b_end];
    let before = &content[..b_start];
    let after = &content[b_end..];

    let new_content = format!("{}{}{}{}{}", before, block_open, selected, block_close, after);
    let open_chars = char_count(block_open);
    let close_chars = char_count(block_close);
    let _ = close_chars;
    let new_start = start + open_chars;
    let new_end = new_start + char_count(selected);
    ActionResult {
        new_content,
        new_cursor_start: new_start,
        new_cursor_end: new_end,
    }
}

/// Move the current line (or selected lines) up or down.
///
/// # Parameters
/// * `content` - The current note text
/// * `sel_start` - Selection start in character indices
/// * `sel_end` - Selection end in character indices
/// * `up` - `true` to move up, `false` to move down
///
/// # Returns
/// `Some(ActionResult)` if the move succeeded, `None` if already at the boundary.
pub fn move_lines(content: &str, sel_start: usize, sel_end: usize, up: bool) -> Option<ActionResult> {
    let (start, end) = if sel_start <= sel_end {
        (sel_start, sel_end)
    } else {
        (sel_end, sel_start)
    };
    let b_start = char_to_byte(content, start);
    let b_end = char_to_byte(content, end);

    let block_start = content[..b_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let block_end = content[b_end..]
        .find('\n')
        .map(|i| b_end + i)
        .unwrap_or(content.len());

    if up {
        if block_start == 0 {
            return None;
        }
        let prev_line_start = content[..block_start - 1]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let prev_line = &content[prev_line_start..block_start];
        let block = &content[block_start..block_end];
        let suffix = &content[block_end..];

        let mut new_content = String::with_capacity(content.len() + 1);
        new_content.push_str(&content[..prev_line_start]);
        new_content.push_str(block);
        new_content.push('\n');
        new_content.push_str(prev_line.strip_suffix('\n').unwrap_or(prev_line));
        new_content.push_str(suffix);

        let shift_chars = char_count(prev_line) as i64;
        let new_start = (sel_start as i64 - shift_chars).max(0) as usize;
        let new_end = (sel_end as i64 - shift_chars).max(0) as usize;
        Some(ActionResult {
            new_content,
            new_cursor_start: new_start,
            new_cursor_end: new_end,
        })
    } else {
        if block_end >= content.len() {
            return None;
        }
        let next_line_end = content[block_end + 1..]
            .find('\n')
            .map(|i| block_end + 1 + i)
            .unwrap_or(content.len());
        let block = &content[block_start..block_end];
        let next_line = &content[block_end + 1..next_line_end];

        let mut new_content = String::with_capacity(content.len() + 1);
        new_content.push_str(&content[..block_start]);
        new_content.push_str(next_line);
        new_content.push('\n');
        new_content.push_str(block);
        if next_line_end < content.len() {
            new_content.push_str(&content[next_line_end..]);
        }

        let shift_chars = char_count(next_line) as i64 + 1;
        let new_start = sel_start + shift_chars as usize;
        let new_end = sel_end + shift_chars as usize;
        Some(ActionResult {
            new_content,
            new_cursor_start: new_start,
            new_cursor_end: new_end,
        })
    }
}

/// Insert a markdown table skeleton at the cursor.
fn insert_table(content: &str, start: usize, end: usize, rows: usize, cols: usize) -> ActionResult {
    let mut table = String::from("\n");
    table.push('|');
    for c in 0..cols {
        table.push_str(&format!(" Header{} |", c + 1));
    }
    table.push('\n');
    table.push('|');
    for _ in 0..cols {
        table.push_str("--------|");
    }
    table.push('\n');
    for _ in 0..rows {
        table.push('|');
        for _ in 0..cols {
            table.push_str("        |");
        }
        table.push('\n');
    }

    insert_text(content, start, end, &table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_toggle() {
        let content = "hello";
        let res = apply(EditorAction::Wrap { prefix: "**", suffix: "**" }, content, 0, 5);
        assert_eq!(res.new_content, "**hello**");
        let res2 = apply(EditorAction::Wrap { prefix: "**", suffix: "**" }, &res.new_content, 0, res.new_content.chars().count());
        assert_eq!(res2.new_content, "hello");
    }

    #[test]
    fn test_line_prefix_toggle() {
        let content = "a\nb\n";
        let res = apply(EditorAction::LinePrefix("- "), content, 0, content.len());
        assert!(res.new_content.contains("- a"));
        assert!(res.new_content.contains("- b"));
        let res2 = apply(EditorAction::LinePrefix("- "), &res.new_content, 0, res.new_content.chars().count());
        assert_eq!(res2.new_content, content);
    }

    #[test]
    fn test_move_lines_up_down() {
        let content = "one\ntwo\nthree\n";
        if let Some(r) = move_lines(content, 4, 7, true) {
            assert!(r.new_content.contains("two\none" ) || r.new_content.contains("two\none"));
        } else {
            panic!("move up failed");
        }
    }
}
