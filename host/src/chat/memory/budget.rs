//! Markdown memory budget enforcement (`MEMORY.md` max 200 lines / 24 KB),
//! secret redaction (`<redacted N chars>`), and automatic rotation to `archive.md`
//! (`docs/memory.md` from `OpenMausBot`).

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

pub const MAX_MEMORY_LINES: usize = 200;
pub const MAX_MEMORY_BYTES: usize = 24 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BudgetLevel {
    Green,
    Amber,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBudgetStatus {
    pub lines: usize,
    pub max_lines: usize,
    pub bytes: usize,
    pub max_bytes: usize,
    pub usage_percent: u8,
    pub level: BudgetLevel,
}

/// Scrubs API keys, tokens, passwords, and private key blocks before writing memory notes to disk.
pub fn redact_secrets(input: &str) -> String {
    let without_pem = redact_pem_blocks(input);
    without_pem.lines().map(redact_line_secrets).collect::<Vec<_>>().join("\n")
}

fn redact_pem_blocks(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("-----BEGIN ") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start..];
        if let Some(end_marker) = after_start.find("-----END ")
            && let Some(close) = after_start[end_marker + 9..].find("-----")
        {
            let total_len = end_marker + 9 + close + 5;
            let _ = write!(out, "<redacted {total_len} chars>");
            rest = &after_start[total_len..];
            continue;
        }
        out.push_str("-----BEGIN ");
        rest = &after_start["-----BEGIN ".len()..];
    }
    out.push_str(rest);
    out
}

fn redact_line_secrets(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    for kw in ["api_key", "api-key", "apikey", "secret", "token", "password"] {
        if let Some(pos) = lower.find(kw) {
            let after_kw = &line[pos + kw.len()..];
            let trimmed = after_kw.trim_start();
            if let Some(sep) = trimmed.chars().next()
                && (sep == ':' || sep == '=')
            {
                let after_sep = trimmed[1..].trim_start();
                let unquoted = after_sep.trim_start_matches(['\'', '"']);
                let secret_len =
                    unquoted.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').count();
                if secret_len >= 8 {
                    let prefix = &line[..pos + kw.len()];
                    return format!("{prefix}{sep} <redacted {secret_len} chars>");
                }
            }
        }
    }
    line.to_owned()
}

/// Computes the visual traffic-light budget indicator for `MEMORY.md` (200 lines / 24 KB).
pub fn evaluate_budget(content: &str) -> MemoryBudgetStatus {
    let lines = content.lines().count();
    let bytes = content.len();
    let line_pct = (lines * 100) / MAX_MEMORY_LINES;
    let byte_pct = (bytes * 100) / MAX_MEMORY_BYTES;
    let pct = u8::try_from(line_pct.max(byte_pct).min(100)).unwrap_or(100);
    let level = if lines > MAX_MEMORY_LINES || bytes > MAX_MEMORY_BYTES {
        BudgetLevel::Red
    } else if pct >= 80 {
        BudgetLevel::Amber
    } else {
        BudgetLevel::Green
    };
    MemoryBudgetStatus {
        lines,
        max_lines: MAX_MEMORY_LINES,
        bytes,
        max_bytes: MAX_MEMORY_BYTES,
        usage_percent: pct,
        level,
    }
}

/// Rotates struck-through (`~~...~~`) or oldest lines out of `MEMORY.md` when it exceeds
/// `MAX_MEMORY_LINES` (200 lines) or `MAX_MEMORY_BYTES` (24 KB).
/// Returns `(kept_memory_md, newly_archived_block)`.
pub fn rotate_memory_if_needed(content: &str, date_ymd: &str) -> (String, Option<String>) {
    let redacted = redact_secrets(content);
    let all_lines: Vec<&str> = redacted.lines().collect();
    if all_lines.len() <= MAX_MEMORY_LINES && redacted.len() <= MAX_MEMORY_BYTES {
        return (redacted, None);
    }

    let mut kept = Vec::new();
    let mut archived = Vec::new();

    // First pass: archive struck-through notes (`~~...~~`)
    for line in all_lines {
        let trimmed = line.trim();
        if trimmed.contains("~~") && trimmed.matches("~~").count() >= 2 {
            archived.push(line.to_owned());
        } else {
            kept.push(line.to_owned());
        }
    }

    // Second pass: if still over budget, evict oldest non-header lines from the top
    let mut idx = 0;
    while (kept.len() > MAX_MEMORY_LINES || kept.iter().map(|l| l.len() + 1).sum::<usize>() > MAX_MEMORY_BYTES)
        && idx < kept.len()
    {
        if kept[idx].trim_start().starts_with('#') || kept[idx].trim().is_empty() {
            idx += 1;
            continue;
        }
        archived.push(kept.remove(idx));
    }

    if archived.is_empty() {
        return (kept.join("\n"), None);
    }

    let archive_block = format!("## [Archivado el {date_ymd}]\n{}\n", archived.join("\n"));
    (kept.join("\n"), Some(archive_block))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets_and_private_keys() {
        let raw = "api_key = \"sk_live_1234567890abcdef\"\nNormal note here.";
        let clean = redact_secrets(raw);
        assert!(!clean.contains("sk_live_1234567890abcdef"));
        assert!(clean.contains("<redacted 24 chars>"));
        assert!(clean.contains("Normal note here."));
    }

    #[test]
    fn rotates_overflow_and_struck_lines_to_archive() {
        let mut lines = vec!["# Memory".to_string(), "- ~~old obsolete fact~~".to_string()];
        for i in 0..205 {
            lines.push(format!("- fact number {i}"));
        }
        let input = lines.join("\n");
        let (kept, archived) = rotate_memory_if_needed(&input, "2026-10-22");
        assert!(kept.lines().count() <= MAX_MEMORY_LINES);
        let arch = archived.expect("should produce archive block");
        assert!(arch.contains("[Archivado el 2026-10-22]"));
        assert!(arch.contains("~~old obsolete fact~~"));
        let status = evaluate_budget(&kept);
        assert!(matches!(status.level, BudgetLevel::Green | BudgetLevel::Amber));
    }
}
