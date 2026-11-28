#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `packages/ui/src/markdown.ts`: the parsing half of its
//! small safe Markdown renderer (inline marks, mentions, wiki links, ids, tables and plain prose).
//! Composes helpers already ported into this host instead of duplicating them. Only `botInsights`
//! calls this.

/// Whether a link target uses a scheme a chat renderer may open (SAFE_URL).
pub fn is_safe_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http:") || lower.starts_with("https:") || lower.starts_with("mailto:")
}

/// Replace every `open`…`close` pair in a string with the result of `fold`.
fn replace_pairs(text: &str, open: &str, close: &str, fold: impl Fn(&str) -> String) -> String {
    let mut out = text.to_owned();
    let mut from = 0;
    while let Some(rel) = out[from..].find(open) {
        let start = from + rel;
        let inner_start = start + open.len();
        let Some(end_rel) = out[inner_start..].find(close) else {
            break;
        };
        let inner_end = inner_start + end_rel;
        let replacement = fold(&out[inner_start..inner_end]);
        out.replace_range(start..inner_end + close.len(), &replacement);
        from = start + replacement.len();
    }
    out
}

/// Code and bold spans of a short line, in order, with the rest as plain text (codeAndBoldParts).
pub fn code_and_bold_parts(text: &str) -> Vec<(String, Option<&'static str>)> {
    let mut out = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let tick = rest.find('`');
        let bold = rest.find("**");
        let (at, mark) = match (tick, bold) {
            (Some(a), Some(b)) if b < a => (b, "**"),
            (Some(a), _) => (a, "`"),
            (None, Some(b)) => (b, "**"),
            (None, None) => break,
        };
        plain.push_str(&rest[..at]);
        let start = at + mark.len();
        if let Some(end) = rest[start..].find(mark) {
            if !plain.is_empty() {
                out.push((std::mem::take(&mut plain), None));
            }
            out.push((rest[start..start + end].to_owned(), Some(if mark == "`" { "code" } else { "bold" })));
            rest = &rest[start + end + mark.len()..];
        } else {
            plain.push_str(&rest[at..start]);
            rest = &rest[start..];
        }
    }
    if !plain.is_empty() {
        out.push((plain, None));
    }
    if !rest.is_empty() {
        out.push((rest.to_owned(), None));
    }
    out
}

/// @handles mentioned in a text, in order, lower-cased and without the `@` (MENTION).
fn mention_handles(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, ch) in text.char_indices() {
        if ch != '@' {
            continue;
        }
        if i > 0 && text.as_bytes()[i - 1].is_ascii_alphanumeric() {
            continue;
        }
        let rest = &text[i + 1..];
        let count = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')).count();
        if count < 2 {
            continue;
        }
        let handle: String = rest.chars().take(count).collect::<String>().to_ascii_lowercase();
        if !out.contains(&handle) {
            out.push(handle);
        }
    }
    out
}

/// Number of distinct @mentions in a text.
pub fn mention_count(text: &str) -> usize {
    mention_handles(text).len()
}

/// Whether a text @mentions a handle, ignoring case (mentionsHandle).
pub fn mentions_handle(text: &str, handle: &str) -> bool {
    let wanted = handle.trim_start_matches('@').to_ascii_lowercase();
    mention_handles(text).iter().any(|h| h == &wanted)
}

/// The first agent handle in `handles` a text mentions, ignoring case (firstMentioned).
pub fn first_mentioned<'a>(text: &str, handles: &'a [&'a str]) -> Option<&'a str> {
    for mention in mention_handles(text) {
        if let Some(handle) = handles.iter().copied().find(|h| h.eq_ignore_ascii_case(&mention)) {
            return Some(handle);
        }
    }
    None
}

/// A wiki token's title and label: `[[Path|label]]` -> (`Path`, `label`) (parseWikiToken).
pub fn parse_wiki_token(token: &str) -> (String, String) {
    let inner = token.strip_prefix("[[").and_then(|t| t.strip_suffix("]]")).unwrap_or(token);
    let (title, label) = match inner.split_once('|') {
        Some((title, label)) => (title.trim(), label.trim()),
        None => (inner.trim(), inner.trim()),
    };
    (title.to_owned(), if label.is_empty() { title.to_owned() } else { label.to_owned() })
}

/// Whether a wiki target reads as a page name rather than a sentence (readsAsPageTitle).
pub fn reads_as_page_title(title: &str) -> bool {
    title.chars().count() <= 80 && !title.trim().ends_with(['.', '!', '?', '…', ':', ';'])
}

/// Every `[[wiki]]` title in a text, in order, without repeats (wikiLinksIn).
pub fn wiki_links_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = text[from..].find("[[") {
        let start = from + rel;
        let Some(end_rel) = text[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + end_rel;
        let (title, _) = parse_wiki_token(&text[start..end + 2]);
        if !title.is_empty() && !out.contains(&title) {
            out.push(title);
        }
        from = end + 2;
    }
    out
}

/// Whether a string is a UUID-shaped internal id.
pub fn is_uuid(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let lengths = [8, 4, 4, 4, 12];
    parts.len() == 5
        && parts.iter().zip(lengths).all(|(p, n)| p.len() == n as usize && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Internal ids in a text: byte position, lower-case id and whether a kind word marks it (internalIds).
pub fn internal_ids(text: &str) -> Vec<(usize, String, bool)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 36 <= bytes.len() {
        let candidate = &text[i..i + 36];
        if is_uuid(candidate)
            && !i.checked_sub(1).and_then(|p| bytes.get(p)).is_some_and(|b| {
                matches!(b, b'/' | b'?' | b'&' | b'=' | b'#' | b'%' | b'.' | b'+' | b'@' | b'~' | b'-')
            })
        {
            let before = text[..i].to_ascii_lowercase();
            let marked = before.ends_with('=') || before.contains("artifact") || before.contains("artefato");
            out.push((i, candidate.to_ascii_lowercase(), marked));
            i += 36;
            continue;
        }
        i += 1;
    }
    out
}

/// Strip a paired inline mark from a string, keeping its inner text.
fn unwrap_pair(text: &str, open: &str, close: &str) -> String {
    text.strip_prefix(open).and_then(|t| t.strip_suffix(close)).map_or_else(|| text.to_owned(), str::to_owned)
}

/// Markdown inline text rendered as prose (plainInline), reusing the shared wiki and span helpers.
pub fn plain_inline(text: &str) -> String {
    let mut out = text.replace("![", "[");
    out = replace_pairs(&out, "[[", "]]", |token| parse_wiki_token(token).1);
    let mut from = 0;
    while let Some(start) = out[from..].find('[') {
        let start = from + start;
        let Some(mid) = out[start..].find("](") else {
            break;
        };
        let label_end = start + mid;
        let Some(end_rel) = out[label_end + 2..].find(')') else {
            break;
        };
        let end = label_end + 2 + end_rel;
        let label = out[start + 1..label_end].to_owned();
        out.replace_range(start..=end, &label);
        from = start + label.len();
    }
    let parts = code_and_bold_parts(&out);
    out = parts.into_iter().map(|(text, _)| text).collect();
    // emphasis left over from a cut message
    for mark in ["~~", "__", "**"] {
        if out.starts_with(mark) && out.ends_with(mark) && out.len() >= 2 * mark.len() {
            out = unwrap_pair(&out, mark, mark);
        }
    }
    out.replace('*', "").replace('`', "")
}

/// Unpaired marks and `<details>` tags left where a text was cut (looseMarks).
pub fn strip_loose_marks(text: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with("**") || text[i..].starts_with("~~") {
            let prev_ws = i == 0 || text.as_bytes()[i - 1].is_ascii_whitespace();
            let after = &text[i + 2..];
            let next_ws = after.is_empty() || after.starts_with(char::is_whitespace);
            if !(prev_ws && next_ws) {
                i += 2;
                continue;
            }
            out.push_str(&text[i..i + 2]);
            i += 2;
            continue;
        }
        let ch = text[i..].chars().next().unwrap_or(' ');
        if ch != '`' {
            out.push(ch);
        }
        i += ch.len_utf8();
    }
    for tag in ["<details>", "</details>", "<summary>", "</summary>", "<DETAILS>", "<SUMMARY>"] {
        out = out.replace(tag, "");
    }
    out
}

/// A list item's text, when the line is one (LIST).
pub fn list_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let hyphen = trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ");
    if hyphen {
        return Some(trimmed[2..].trim());
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && trimmed[digits..].starts_with(". ") || digits > 0 && trimmed[digits..].starts_with(") ") {
        return Some(trimmed[digits + 2..].trim());
    }
    None
}

/// A horizontal or table rule made of dashes (TABLE_RULE).
pub fn is_rule_line(line: &str) -> bool {
    let core = line.trim().trim_start_matches('|').trim_end_matches('|').trim();
    let dashes = core.chars().filter(|c| *c == '-').count();
    dashes >= 3 && core.chars().all(|c| c == '-' || c == ':' || c.is_whitespace())
}

/// A fenced code delimiter (FENCE).
pub fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

/// A quote line.
pub fn is_quote(line: &str) -> bool {
    line.trim_start().starts_with('>')
}

/// A heading's level, 1 to 6, when the line is one.
pub fn head_level(line: &str) -> Option<u8> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && line.chars().nth(hashes) == Some(' ') { Some(hashes as u8) } else { None }
}

/// A stable anchor id for a heading (headingId), reusing the shared slugifier.
pub fn heading_id(text: &str) -> String {
    crate::bdots_textfmt::slugify(text)
}

/// Cells of a table row, trimmed of surrounding pipes and spaces (splitRow).
pub fn split_row(line: &str) -> Vec<String> {
    let core = line.trim().trim_start_matches('|').trim_end_matches('|');
    core.split('|').map(|cell| cell.trim().to_owned()).collect()
}

/// Whether a table cell reads as a number (NUMERIC), so its column aligns right.
pub fn is_numeric_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    let mut body = trimmed.trim_start_matches(|c: char| matches!(c, '~' | '%' | '^' | '<' | '>' | '+'));
    body = body.trim_start_matches("US$").trim_start_matches("R$").trim_start_matches('$').trim_start_matches('€');
    body = body.trim_start_matches(['+', '-']);
    let mut digits = 0;
    let mut ok = true;
    for c in body.chars() {
        if c.is_ascii_digit() {
            digits += 1;
        } else if !matches!(c, '.' | ',' | ' ' | '%' | 'k' | 'x') && !c.is_alphabetic() {
            ok = false;
            break;
        }
    }
    digits > 0 && ok
}

/// Alignment of each table column from its rule and numeric cells (tableAt).
pub fn table_align(marks: &[String], rows: &[Vec<String>]) -> Vec<&'static str> {
    marks
        .iter()
        .enumerate()
        .map(|(col, mark)| {
            if mark.starts_with(':') && mark.ends_with(':') {
                "center"
            } else if mark.ends_with(':') {
                "right"
            } else if mark.starts_with(':') {
                "left"
            } else {
                let cells: Vec<&str> =
                    rows.iter().filter_map(|r| r.get(col)).map(String::as_str).filter(|c| !c.is_empty()).collect();
                if !cells.is_empty() && cells.iter().all(|c| is_numeric_cell(c)) { "right" } else { "left" }
            }
        })
        .collect()
}

/// A `<details>` line's summary, when the line opens a collapsed block (DETAILS_OPEN).
pub fn details_summary(line: &str) -> Option<String> {
    let lower = line.trim_start().to_ascii_lowercase();
    if !lower.starts_with("<details>") {
        return None;
    }
    Some(
        line.trim_start()["<details>".len()..]
            .trim()
            .strip_prefix("<summary>")
            .and_then(|rest| rest.split_once("</summary>"))
            .map_or_else(String::new, |(summary, _)| summary.trim().to_owned()),
    )
}

/// Prose lines of a Markdown text: paragraphs, list items and quotes only (plainLines).
pub fn plain_lines(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if is_fence(line) {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if trimmed.is_empty()
            || head_level(line).is_some()
            || is_fence(line)
            || is_rule_line(line)
            || trimmed.starts_with('|')
            || trimmed.starts_with("![")
        {
            continue;
        }
        let body = list_text(line).or_else(|| line.trim_start().strip_prefix('>').map(str::trim)).unwrap_or(trimmed);
        let plain = strip_loose_marks(&plain_inline(body));
        let collapsed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
        if !collapsed.is_empty() {
            out.push(collapsed);
        }
    }
    out
}

/// One line of Markdown prose, bounded to `shown` characters (plainLine).
pub fn plain_line(source: &str, shown: usize) -> String {
    crate::bdots_activity::shorten(&plain_lines(source).join(" "), shown)
}

/// Whether a Markdown text carries a block wider than prose (hasWideBlock).
pub fn has_wide_block(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.trim();
        head_level(line).is_some()
            || is_fence(line)
            || is_rule_line(line)
            || trimmed.starts_with('|')
            || trimmed.starts_with("![")
    })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(blocks: usize, links: usize, mentions: usize, wide: bool) -> Vec<(&'static str, String)> {
    vec![
        ("blocks", count_label(blocks)),
        ("links", count_label(links)),
        ("mentions", count_label(mentions)),
        ("wide", if wide { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_marks_and_links() {
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("mailto:a@b.com"));
        assert!(!is_safe_url("javascript:alert(1)"));
        let parts = code_and_bold_parts("use `x` and **y** now");
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[1], ("x".to_owned(), Some("code")));
        assert_eq!(parts[3], ("y".to_owned(), Some("bold")));
        assert_eq!(plain_inline("`code` and **bold**"), "code and bold");
        assert_eq!(plain_inline("[label](https://x.com)"), "label");
        assert_eq!(plain_inline("[[Path/Page|read this]]"), "read this");
    }

    #[test]
    fn mentions_and_wiki() {
        assert!(mentions_handle("ask @Alice please", "alice"));
        assert!(!mentions_handle("ana@example.com", "example"));
        assert_eq!(mention_count("@bob and @alice"), 2);
        assert_eq!(first_mentioned("hi @bob and @alice", &["alice", "bob"]), Some("bob"));
        assert_eq!(parse_wiki_token("[[Docs/Intro|the intro]]").0, "Docs/Intro");
        assert!(reads_as_page_title("Introduction"));
        assert!(!reads_as_page_title("A sentence that ends."));
        assert_eq!(wiki_links_in("[[A]] and [[B|label]] and [[A]]"), vec!["A", "B"]);
    }

    #[test]
    fn ids_and_lines() {
        let id = "0193f4a2-1234-4abc-8def-0123456789ab";
        assert!(is_uuid(id));
        assert!(!is_uuid("not-an-id"));
        let found = internal_ids(&format!("print, artifact {id}"));
        assert_eq!(found.len(), 1);
        assert!(found[0].2);
        assert_eq!(list_text("- first"), Some("first"));
        assert_eq!(list_text("2. second"), Some("second"));
        assert_eq!(list_text("plain"), None);
        assert!(is_rule_line("|---|---|"));
        assert!(is_fence("  ```rust"));
        assert!(is_quote("> quoted"));
        assert_eq!(head_level("## Title"), Some(2));
        assert_eq!(heading_id("Hello World!"), "hello-world");
    }

    #[test]
    fn tables_and_details() {
        assert_eq!(split_row("| a | b |"), vec!["a", "b"]);
        assert!(is_numeric_cell("-17,2%"));
        assert!(!is_numeric_cell("hello"));
        let marks = vec![":---".to_owned(), "---:".to_owned(), ":---:".to_owned()];
        assert_eq!(table_align(&marks, &[]), vec!["left", "right", "center"]);
        assert_eq!(details_summary("<details><summary>More</summary>"), Some("More".to_owned()));
        assert_eq!(details_summary("plain"), None);
    }

    #[test]
    fn prose_and_summary() {
        let source = "# Title\n\nHello **world**.\n\n- first item\n- [second](https://x.com)\n\n```\ncode\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |";
        let lines = plain_lines(source);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Hello world.");
        assert_eq!(lines[1], "first item");
        assert_eq!(lines[2], "second");
        assert!(has_wide_block(source));
        assert_eq!(plain_line(source, 8).len(), 8);
        assert_eq!(strip_loose_marks("**bold** and 2 ** 3"), "bold and 2 ** 3");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, 1, true).len(), 4);
    }
}
