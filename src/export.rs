//! HTML export from Markdown using `pulldown_cmark`.

use pulldown_cmark::{html, Options, Parser};

/// Convert markdown text to a full HTML document with embedded CSS.
///
/// # Parameters
/// * `markdown` - The raw markdown content
/// * `title` - Document title (used in `<title>` tag)
///
/// # Returns
/// A complete HTML page string
pub fn markdown_to_html(markdown: &str, title: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);
    let mut body = String::new();
    html::push_html(&mut body, parser);

    let body = add_heading_ids(&body);

    let css = include_str!("../assets/export.css");
    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{}</title>
<style>
{}
</style>
</head>
<body>
<article class="markdown-body">
{}
</article>
</body>
</html>
"#,
        html_escape(title),
        css,
        body
    )
}

/// Escape HTML special characters in a string.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Walk rendered HTML and inject `id="..."` attributes on `<hN>` headings
/// so that intra-document anchor links resolve.
fn add_heading_ids(html: &str) -> String {
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len() + 64);
    let mut i = 0;
    while i < bytes.len() {
        if i + 3 < bytes.len()
            && bytes[i] == b'<'
            && bytes[i + 1] == b'h'
            && (b'1'..=b'6').contains(&bytes[i + 2])
            && bytes[i + 3] == b'>'
        {
            let level = bytes[i + 2] as char;
            let close_tag = format!("</h{}>", level);
            if let Some(rel) = html[i + 4..].find(&close_tag) {
                let inner_start = i + 4;
                let inner_end = i + 4 + rel;
                let inner = &html[inner_start..inner_end];
                let plain = strip_html_tags(inner);
                let id = html_escape(plain.trim());
                out.push_str(&format!("<h{} id=\"{}\">", level, id));
                out.push_str(inner);
                out.push_str(&close_tag);
                i = inner_end + close_tag.len();
                continue;
            }
        }
        let ch_start = i;
        let mut ch_end = i + 1;
        while ch_end < bytes.len() && (bytes[ch_end] & 0xC0) == 0x80 {
            ch_end += 1;
        }
        out.push_str(&html[ch_start..ch_end]);
        i = ch_end;
    }
    out
}

/// Strip HTML tags from a string, keeping only the text content.
fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}
