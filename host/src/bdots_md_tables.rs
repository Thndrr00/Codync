#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `markdown_tables.py`: CJK-aware markdown table
//! detection, display width, wrapping and re-alignment. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

/// Minimum width of a table column (the divider's minimum dash run).
pub const MIN_COL_WIDTH: usize = 3;

/// Split `| a | b |` into trimmed cells (split_table_row).
pub fn split_table_row(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let core = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let core = core.strip_suffix('|').unwrap_or(core);
    core.split('|').map(|cell| cell.trim().to_owned()).collect()
}

/// Whether a cell is a divider cell (`:---:`).
pub fn is_divider_cell(cell: &str) -> bool {
    let core = cell.trim().trim_start_matches(':').trim_end_matches(':');
    core.len() >= 3 && core.chars().all(|c| c == '-')
}

/// Whether a row is a markdown table divider (is_table_divider).
pub fn is_table_divider(row: &str) -> bool {
    let cells = split_table_row(row);
    cells.len() > 1 && cells.iter().all(|cell| is_divider_cell(cell))
}

/// Whether a row could plausibly be a table row (looksLikeTableRow).
pub fn looks_like_table_row(row: &str) -> bool {
    let trimmed = row.trim();
    !trimmed.is_empty() && (trimmed.starts_with('|') || trimmed.matches('|').count() >= 2)
}

/// Display width of a string: wide CJK/emoji count as two cells (_disp_width).
pub fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

/// Display width of a character, clamped to >= 0.
pub fn char_width(c: char) -> usize {
    let code = c as u32;
    if code < 32 || (0x7f..0xa0).contains(&code) {
        0
    } else if is_wide(code) {
        2
    } else {
        1
    }
}

/// Whether a code point occupies two terminal cells.
fn is_wide(code: u32) -> bool {
    matches!(code,
        0x1100..=0x115f | 0x2329..=0x232a | 0x2e80..=0xa4cf | 0xac00..=0xd7a3 |
        0xf900..=0xfaff | 0xfe10..=0xfe19 | 0xfe30..=0xfe6f | 0xff00..=0xff60 |
        0xffe0..=0xffe6 | 0x1f300..=0x1f64f | 0x1f900..=0x1f9ff | 0x20000..=0x3fffd)
}

/// Pad a cell on the right to a given display width.
pub fn pad_cell(cell: &str, width: usize) -> String {
    let used = display_width(cell);
    format!("{}{}", cell, " ".repeat(width.saturating_sub(used)))
}

/// Render a table block at uniform widths (_renderBlock).
pub fn render_block(rows: &[Vec<String>]) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }
    let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..ncols)
        .map(|col| {
            rows.iter()
                .map(|row| row.get(col).map_or(MIN_COL_WIDTH, |cell| display_width(cell).max(MIN_COL_WIDTH)))
                .max()
                .unwrap_or(MIN_COL_WIDTH)
        })
        .collect();
    let render_row = |row: &Vec<String>| {
        let cells: String = (0..ncols)
            .map(|col| pad_cell(row.get(col).map_or("", String::as_str), widths[col]))
            .collect::<Vec<_>>()
            .join(" | ");
        format!("| {cells} |")
    };
    let divider = format!("|{}|", widths.iter().map(|w| "-".repeat(w + 2)).collect::<Vec<_>>().join("|"));
    let mut out = vec![render_row(&rows[0]), divider];
    out.extend(rows.iter().skip(1).map(render_row));
    out
}

/// Hard-break a word into chunks of at most `width` display cells (_hardBreak).
pub fn hard_break(word: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![word.to_owned()];
    }
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut used = 0;
    for c in word.chars() {
        let cw = char_width(c).max(1);
        if used + cw > width && !buf.is_empty() {
            out.push(std::mem::take(&mut buf));
            used = 0;
        }
        buf.push(c);
        used += cw;
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

/// Soft-wrap text at word boundaries, hard-breaking long words (_wrapToWidth).
pub fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    if width == 0 || text.is_empty() {
        return vec![text.to_owned()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut used = 0;
    for word in text.split_whitespace() {
        let w = display_width(word);
        if used > 0 && used + 1 + w > width {
            lines.push(std::mem::take(&mut current));
            used = 0;
        }
        if w > width {
            for chunk in hard_break(word, width) {
                lines.push(chunk);
            }
            continue;
        }
        if used > 0 {
            current.push(' ');
            used += 1;
        }
        current.push_str(word);
        used += w;
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

/// Render a narrow block as vertical key/value lines (_renderVertical).
pub fn render_vertical(rows: &[Vec<String>]) -> Vec<String> {
    let Some(header) = rows.first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows.iter().skip(1) {
        for (col, key) in header.iter().enumerate() {
            let value = row.get(col).map_or("", String::as_str);
            out.push(format!("{key}: {value}"));
        }
        out.push(String::new());
    }
    out
}

/// Whether a table block exceeds an available width.
pub fn too_wide(rows: &[Vec<String>], available_width: usize) -> bool {
    let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let total: usize = (0..ncols)
        .map(|col| {
            rows.iter()
                .map(|row| row.get(col).map_or(MIN_COL_WIDTH, |cell| display_width(cell).max(MIN_COL_WIDTH)))
                .max()
                .unwrap_or(MIN_COL_WIDTH)
        })
        .sum();
    total + 3 * ncols + 1 > available_width.max(20)
}

/// Re-align markdown tables in a text block (realign_markdown_tables).
pub fn realign_tables(lines: &[&str], available_width: Option<usize>) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if i + 1 < lines.len() && looks_like_table_row(lines[i]) && is_table_divider(lines[i + 1]) {
            let mut rows = vec![split_table_row(lines[i])];
            i += 2;
            while i < lines.len() && looks_like_table_row(lines[i]) {
                rows.push(split_table_row(lines[i]));
                i += 1;
            }
            let block = if available_width.is_some_and(|w| too_wide(&rows, w)) {
                render_vertical(&rows)
            } else {
                render_block(&rows)
            };
            out.extend(block);
            continue;
        }
        out.push(lines[i].to_owned());
        i += 1;
    }
    out
}

/// Number of columns in a table row.
pub fn table_columns(row: &str) -> usize {
    split_table_row(row).len()
}

/// Whether a text block contains a table-like row.
pub fn has_table_block(lines: &[&str]) -> bool {
    lines.iter().any(|line| looks_like_table_row(line))
}

/// Join cells into a single pipe-delimited row.
pub fn join_cells(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

/// Alignment declared by a divider cell.
pub fn alignment_of(cell: &str) -> &'static str {
    let trimmed = cell.trim();
    let left = trimmed.starts_with(':');
    let right = trimmed.ends_with(':');
    match (left, right) {
        (true, true) => "center",
        (false, true) => "right",
        (true, false) => "left",
        _ => "none",
    }
}

/// Divider line for a given number of columns.
pub fn divider_for(columns: usize, alignment: &str) -> String {
    let cell = match alignment {
        "center" => ":---:",
        "right" => "---:",
        "left" => ":---",
        _ => "---",
    };
    format!("|{}|", vec![cell; columns.max(1)].join("|"))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(rows: usize, cols: usize, wide: usize, narrow: bool) -> Vec<(&'static str, String)> {
    vec![
        ("rows", count_label(rows)),
        ("cols", count_label(cols)),
        ("wide", count_label(wide)),
        ("layout", if narrow { "vertical" } else { "horizontal" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_dividers() {
        assert_eq!(split_table_row("| a | b |"), vec!["a", "b"]);
        assert!(is_divider_cell(":---:"));
        assert!(is_table_divider("| --- | :---: |"));
        assert!(looks_like_table_row("| a | b |"));
        assert!(looks_like_table_row("a | b"));
        assert!(!looks_like_table_row("plain"));
    }

    #[test]
    fn widths_and_padding() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("中文"), 4);
        assert_eq!(char_width('A'), 1);
        assert_eq!(char_width('\n'), 0);
        assert_eq!(pad_cell("ab", 5), "ab   ");
        assert_eq!(pad_cell("中文", 5), "中文   ");
    }

    #[test]
    fn rendering() {
        let rows = vec![vec!["a".to_owned(), "b".to_owned()], vec!["1".to_owned(), "2".to_owned()]];
        let rendered = render_block(&rows);
        assert_eq!(rendered.len(), 3);
        assert!(rendered[0].starts_with("| a"));
        assert!(rendered[1].contains("---"));
        assert!(too_wide(&rows, 10));
        assert!(!too_wide(&rows, 80));
    }

    #[test]
    fn wrapping() {
        assert_eq!(hard_break("abcdef", 2), vec!["ab", "cd", "ef"]);
        assert_eq!(wrap_to_width("hello world", 5), vec!["hello", "world"]);
        assert_eq!(wrap_to_width("", 5), vec![""]);
        let vertical = render_vertical(&[vec!["k".to_owned()], vec!["v".to_owned()]]);
        assert_eq!(vertical[0], "k: v");
    }

    #[test]
    fn realigning_and_summary() {
        let lines = ["| a | b |", "|---|---|", "| 1 | 2 |"];
        let aligned = realign_tables(&lines, None);
        assert_eq!(aligned.len(), 3);
        assert!(aligned[0].contains("| a"));
        assert_eq!(realign_tables(&["plain"], None), vec!["plain"]);
        assert_eq!(table_columns("| a | b |"), 2);
        assert!(has_table_block(&["plain", "| a | b |"]));
        assert_eq!(join_cells(&["a".to_owned(), "b".to_owned()]), "| a | b |");
        assert_eq!(alignment_of(":---:"), "center");
        assert_eq!(alignment_of("---:"), "right");
        assert_eq!(divider_for(2, "left"), "|:---|:---|");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, 1, false).len(), 4);
    }
}
