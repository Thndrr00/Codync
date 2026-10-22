//! Error signatures: the same failure, told apart from its paths, numbers and spacing, so repeats match.

use sha2::{Digest, Sha256};

/// Characters of error text kept in a signature and shown to the user.
pub const ERROR_TEXT_MAX: usize = 200;

/// Error text with paths as `<path>` and runs of digits as `<n>`, whitespace folded and clipped to
/// [`ERROR_TEXT_MAX`] characters (ending in `…` when clipped).
pub fn normalize(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        if is_path(word) {
            out.push_str("<path>");
            continue;
        }
        let mut in_digits = false;
        for c in word.chars() {
            if c.is_ascii_digit() {
                if !in_digits {
                    out.push_str("<n>");
                }
                in_digits = true;
            } else {
                out.push(c);
                in_digits = false;
            }
        }
    }
    match out.char_indices().nth(ERROR_TEXT_MAX) {
        Some((cut, _)) => format!("{}…", &out[..cut]),
        None => out,
    }
}

/// A short, stable id for one kind of failure: `<kind>:<16 hex digits>`.
pub fn signature(kind: &str, normalized: &str) -> String {
    let digest = Sha256::digest(format!("{kind}\n{normalized}").as_bytes());
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!("{kind}:{hex}")
}

/// A POSIX or Windows path, or a URL: `/usr/lib`, `src/main.rs`, `C:\Users`, `https://…`.
fn is_path(word: &str) -> bool {
    let word = word.trim_matches(|c: char| matches!(c, '\'' | '"' | '`' | '(' | ')' | ',' | ':' | ';'));
    let drive = word.len() > 2 && word.as_bytes()[1] == b':' && word.as_bytes()[0].is_ascii_alphabetic();
    word.len() > 1 && (word.contains('/') || word.contains('\\') || drive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_numbers_and_spacing_do_not_split_a_failure() {
        let a = normalize("error[E0308]: mismatched types at /home/me/app/src/main.rs:12:5");
        let b = normalize("error[E0308]:  mismatched types at  C:\\work\\app\\src\\lib.rs:40:1");
        assert_eq!(a, "error[E<n>]: mismatched types at <path>");
        assert_eq!(a, b);
        assert_eq!(signature("tool_error", &a), signature("tool_error", &b));
        assert_ne!(signature("tool_error", &a), signature("turn_error", &a));
    }

    #[test]
    fn signatures_are_short_and_stable() {
        let sig = signature("tool_error", "boom");
        assert_eq!(sig.len(), "tool_error:".len() + 16);
        assert_eq!(sig, signature("tool_error", "boom"));
    }

    #[test]
    fn long_errors_are_clipped() {
        assert_eq!(normalize(&"x".repeat(500)).chars().count(), ERROR_TEXT_MAX + 1);
        assert_eq!(normalize("  line 12\n\tcol 7 "), "line <n> col <n>");
    }
}
