#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure text helpers ported from bdots' `pipeline-summary.ts`, `call-preview.ts`, `show.ts`
//! and `choices.ts`. No client, DB or model dependency; each is deterministic and tested.

const MAX_SUMMARY: usize = 160;

/// The first sentence of a reply as plain text, for a one-line step summary. Skips rules, tables
/// and heading-only noise (`bdots/src/lib/pipeline-summary.ts`).
pub fn summarize(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| {
            !l.is_empty()
                && !l.starts_with("---")
                && !l.starts_with("***")
                && !l.starts_with('|')
                && !l.starts_with("```")
        })
        .unwrap_or("");
    let plain = strip_markdown(line);
    let end =
        plain.char_indices().find(|(i, c)| (*c == '.' || *c == '!' || *c == '?') && plain[*i + 1..].starts_with(' '));
    let sentence = match end {
        Some((i, c)) => &plain[..i + c.len_utf8()],
        None => &plain,
    };
    if sentence.chars().count() > MAX_SUMMARY {
        let cut: String = sentence.chars().take(MAX_SUMMARY - 1).collect();
        format!("{}…", cut.trim_end())
    } else {
        sentence.to_owned()
    }
}

fn strip_markdown(line: &str) -> String {
    let mut out = String::new();
    let mut in_link = false;
    let mut link_text = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            i += 1;
            in_link = true;
            link_text.clear();
        } else if c == '[' {
            in_link = true;
            link_text.clear();
        } else if c == ']' && in_link {
            out.push_str(&link_text);
            in_link = false;
        } else if c == '(' && in_link {
            while i < chars.len() && chars[i] != ')' {
                i += 1;
            }
        } else if in_link {
            link_text.push(c);
        } else if c == '#' {
            while i < chars.len() && chars[i] == '#' {
                i += 1;
            }
            while i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            continue;
        } else if matches!(c, '*' | '_' | '`' | '~') {
            // Inline emphasis markers never show.
        } else if matches!(c, '-' | '*' | '+' | '>') && i == 0 {
            // A leading list marker.
        } else {
            out.push(c);
        }
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// "42s", "1m 22s", "2h 5m" (`pipeline-summary.ts`).
pub fn format_span(ms: i64) -> String {
    let s = (ms / 1000).max(0);
    if s < 60 {
        return format!("{s}s");
    }
    let m = s / 60;
    if m < 60 {
        return format!("{}m {}s", m, s % 60);
    }
    format!("{}h {}m", m / 60, m % 60)
}

/// A summary that says the step had nothing to do, so the run can play it down.
pub fn is_noop(summary: &str) -> bool {
    let lower = summary.to_ascii_lowercase();
    ["no ", "nothing", "none"].iter().any(|p| lower.starts_with(p))
}

/// `sentAt`, `sent_at` and `sent-at` all become "Sent at" (`call-preview.ts`).
pub fn human_key(key: &str) -> String {
    const LABELS: &[(&str, &str)] = &[
        ("to", "To"),
        ("cc", "Cc"),
        ("bcc", "Bcc"),
        ("url", "Page"),
        ("path", "File"),
        ("command", "Command"),
        ("subject", "Subject"),
        ("body", "Message"),
        ("text", "Message"),
        ("content", "Content"),
        ("message", "Message"),
        ("query", "Search for"),
        ("prompt", "Instructions"),
        ("name", "Name"),
        ("when", "When"),
        ("cron", "Repeats"),
        ("at", "At"),
    ];
    if let Some((_, label)) = LABELS.iter().find(|(k, _)| *k == key) {
        return (*label).to_owned();
    }
    let mut words = String::new();
    for (i, c) in key.chars().enumerate() {
        if c == '_' || c == '-' {
            if !words.ends_with(' ') {
                words.push(' ');
            }
        } else if c.is_uppercase()
            && i > 0
            && key.chars().nth(i - 1).is_some_and(|p| p.is_lowercase() || p.is_ascii_digit())
        {
            words.push(' ');
            words.extend(c.to_lowercase());
        } else {
            words.extend(c.to_lowercase());
        }
    }
    let words = words.trim();
    let mut chars = words.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// `tool_ab12cd_send_email` is the tool `send_email` (`call-preview.ts`).
pub fn bare_name(name: &str) -> &str {
    if let Some(rest) = name.strip_prefix("tool_")
        && let Some((_, tool)) = rest.split_once('_')
    {
        return tool;
    }
    name
}

/// The letter key shown beside a choice option: A, B, C… (`choices.ts`).
pub fn choice_key(index: usize) -> String {
    let letter = (b'A' + (index.min(25) as u8)) as char;
    if index < 26 { letter.to_string() } else { format!("{}{}", letter, index - 25) }
}

/// Keep "mid-market" on one line: hyphens inside words do not break (`show.ts`).
pub fn keep_hyphens(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    for i in 0..chars.len() {
        out.push(chars[i]);
        if chars[i] == '-'
            && i > 0
            && i + 1 < chars.len()
            && chars[i - 1].is_alphanumeric()
            && chars[i + 1].is_alphanumeric()
        {
            out.pop();
            out.push('\u{2011}');
        }
    }
    out
}

/// The colour of a stat's value: the tone the bot gave it, or else its sign (`show.ts`).
pub fn stat_tone(tone: Option<&str>, value: Option<&str>) -> &'static str {
    match tone {
        Some("neutral") => return "neutral",
        Some("good") => return "good",
        Some("warn") => return "warn",
        _ => {}
    }
    let value = value.unwrap_or("").trim();
    if value.starts_with('-') && value[1..].trim_start().starts_with(|c: char| c.is_ascii_digit()) {
        "warn"
    } else if value.starts_with('+') && value[1..].trim_start().starts_with(|c: char| c.is_ascii_digit()) {
        "good"
    } else {
        "neutral"
    }
}

/// One size for every value in a row of stats, small enough for the longest (`show.ts`).
pub fn stat_size(values: &[&str]) -> &'static str {
    let longest = values.iter().map(|v| v.chars().count()).max().unwrap_or(0);
    if longest <= 8 {
        "lg"
    } else if longest <= 12 {
        "md"
    } else {
        "sm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_takes_the_first_real_sentence() {
        assert_eq!(summarize("---\n# Report\nRevenue grew 12%. Other details."), "Revenue grew 12%.");
        assert_eq!(summarize("| a | b |\nDeployed the fix."), "Deployed the fix.");
        assert_eq!(summarize("**Bold** and `code` here"), "Bold and code here");
        let long = format!("{} more", "word ".repeat(60));
        assert!(summarize(&long).ends_with('…'));
    }

    #[test]
    fn spans_and_noop() {
        assert_eq!(format_span(42_000), "42s");
        assert_eq!(format_span(82_000), "1m 22s");
        assert_eq!(format_span(7_265_000), "2h 1m");
        assert!(is_noop("Nothing sent."));
        assert!(!is_noop("Sent the email."));
    }

    #[test]
    fn keys_names_and_hyphens() {
        assert_eq!(human_key("sent_at"), "Sent at");
        assert_eq!(human_key("sentAt"), "Sent at");
        assert_eq!(human_key("url"), "Page");
        assert_eq!(bare_name("tool_ab12cd_send_email"), "send_email");
        assert_eq!(choice_key(0), "A");
        assert_eq!(choice_key(3), "D");
        assert_eq!(keep_hyphens("mid-market deal"), "mid\u{2011}market deal");
    }

    #[test]
    fn stat_tone_and_size() {
        assert_eq!(stat_tone(Some("warn"), Some("+5")), "warn");
        assert_eq!(stat_tone(None, Some("-12%")), "warn");
        assert_eq!(stat_tone(None, Some("+12%")), "good");
        assert_eq!(stat_tone(None, Some("flat")), "neutral");
        assert_eq!(stat_size(&["12345678"]), "lg");
        assert_eq!(stat_size(&["1234567890123"]), "md");
        assert_eq!(stat_size(&["123456789012345"]), "sm");
    }
}
