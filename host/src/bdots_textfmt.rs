#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat 0.7.5 (`client/lib/text.ts` and `adapters/html-text.ts`):
//! display slugs and handles, address parsing, password-shape checks, amount parsing, sentence and
//! word helpers, and outside-text sanitisation. Composes helpers already ported into this host
//! instead of duplicating them. Only the `botInsights` endpoint calls this.

/// Fold a common accented Latin letter to its ASCII base (the NFD step of `slugify`).
fn fold_accent(c: char) -> char {
    match c {
        'à'..='å' | 'ā'..='ą' => 'a',
        'ç' | 'ć'..='č' => 'c',
        'ď' | 'đ' => 'd',
        'è'..='ë' | 'ē'..='ę' => 'e',
        'ğ' | 'ĝ'..='ģ' => 'g',
        'ĥ' | 'ħ' => 'h',
        'ì'..='ï' | 'ī'..='į' => 'i',
        'ĵ' => 'j',
        'ķ' => 'k',
        'ĺ'..='ļ' => 'l',
        'ñ' | 'ń'..='ň' => 'n',
        'ò'..='ö' | 'ō'..='ő' => 'o',
        'ŕ'..='ř' => 'r',
        'ś'..='ş' => 's',
        'ť' | 'ţ' | 'þ' => 't',
        'ù'..='ü' | 'ū'..='ű' => 'u',
        'ý' | 'ÿ' | 'ŷ' => 'y',
        'ż'..='ź' | 'ž' => 'z',
        _ => c,
    }
}

/// A display slug from a name, e.g. `Acme Saúde` -> `acme-saude` (slugify).
pub fn slugify(value: &str) -> String {
    let mut out = String::new();
    for c in value.trim().to_lowercase().chars() {
        let c = fold_accent(c);
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// An agent @handle from a name: 2–32 lowercase letters, digits, `-` or `_` (handleFrom).
pub fn handle_from(name: &str) -> String {
    let mut out: String =
        name.trim().to_lowercase().chars().map(fold_accent).filter(|c| c.is_ascii_alphanumeric()).take(32).collect();
    while out.len() < 2 {
        out.push('0');
    }
    out
}

/// Whether an address local part or domain label has no wrapper characters (EMAIL pattern).
pub fn valid_part(part: &str) -> bool {
    !part.is_empty()
        && !part
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '@' | ',' | ';' | ':' | '<' | '>' | '(' | ')' | '[' | ']' | '"'))
}

/// Whether a value is a plain email address (isEmail).
pub fn is_email(value: &str) -> bool {
    let value = value.trim();
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    valid_part(local)
        && domain.contains('.')
        && domain
            .split('.')
            .all(|label| valid_part(label) && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

/// Unwrap one token pasted from mail or a spreadsheet into a lower-case address.
fn piece_email(piece: &str) -> String {
    let trimmed =
        piece.trim().trim_matches(|c: char| matches!(c, '<' | '>' | '"' | '\'' | '(' | ')' | '[' | ']' | '.'));
    let bare = trimmed.get(..7).filter(|p| p.eq_ignore_ascii_case("mailto:")).map_or(trimmed, |_| &trimmed[7..]);
    bare.trim().to_ascii_lowercase()
}

/// Addresses typed or pasted, in order and once each, each marked valid (addressesIn).
pub fn addresses_in(text: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for piece in text.split(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | ',' | ';' | '<' | '>' | '"')) {
        let email = piece_email(piece);
        if email.is_empty() || !email.contains('@') || out.iter().any(|(seen, _)| seen == &email) {
            continue;
        }
        let valid = is_email(&email);
        out.push((email, valid));
    }
    out
}

/// Addresses split into valid and invalid, leaving out the ones already known (emailsFrom).
pub fn emails_from(text: &str, already: &[String]) -> (Vec<String>, Vec<String>) {
    let known: Vec<String> = already.iter().map(|e| e.to_ascii_lowercase()).collect();
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    for (email, ok) in addresses_in(text) {
        if known.contains(&email) {
            continue;
        }
        if ok {
            valid.push(email);
        } else {
            invalid.push(email);
        }
    }
    (valid, invalid)
}

/// Password length state from Better Auth rules (passwordState).
pub fn password_state(password: &str) -> &'static str {
    let len = password.chars().count();
    if len == 0 {
        "empty"
    } else if len < 8 {
        "short"
    } else if len > 128 {
        "long"
    } else {
        "ok"
    }
}

/// Whether a name step holds a pasted password (looksLikePassword).
pub fn looks_like_password(name: &str) -> bool {
    let n = name.trim();
    n.chars().count() >= 8
        && !n.chars().any(char::is_whitespace)
        && n.chars().any(|c| c.is_ascii_digit())
        && n.chars().any(|c| !c.is_alphanumeric() && !matches!(c, '-' | '_' | '.' | '\''))
}

/// Whether the chosen password is the name typed a step before (nameIsPassword).
pub fn name_is_password(name: &str, password: &str) -> bool {
    let n = name.trim();
    !n.is_empty() && n == password.trim()
}

/// A decimal amount written with either separator (parseAmount).
pub fn parse_amount(value: &str) -> Option<f64> {
    let normalized: String =
        value.chars().filter(|c| !c.is_whitespace()).map(|c| if c == ',' { '.' } else { c }).collect();
    let mut dots = 0;
    for c in normalized.chars() {
        if c == '.' {
            dots += 1;
            if dots > 1 {
                return None;
            }
        } else if !c.is_ascii_digit() {
            return None;
        }
    }
    normalized.parse::<f64>().ok()
}

/// A suggested correction for a common webmail domain typo (webmailOf).
pub fn email_typo_fix(email: &str) -> Option<String> {
    let (local, domain) = email.trim().split_once('@')?;
    let fixed = match domain.to_ascii_lowercase().as_str() {
        "gmial.com" | "gmai.com" | "gmal.com" | "gamil.com" | "gnail.com" | "gmail.con" | "gmail.co"
        | "gmail.com.br" => "gmail.com",
        "hotmial.com" | "hotmai.com" | "hotmail.con" => "hotmail.com",
        _ => return None,
    };
    Some(format!("{local}@{fixed}"))
}

/// Text ending in a sentence-closing mark (endSentence).
pub fn end_sentence(text: &str) -> String {
    if text.ends_with(['.', '!', '?']) { text.to_owned() } else { format!("{text}.") }
}

/// Text around the first occurrence of a part (around).
pub fn around<'a>(text: &'a str, part: &str) -> Option<(&'a str, &'a str)> {
    let at = text.find(part)?;
    Some((&text[..at], &text[at + part.len()..]))
}

/// Index of the first whole word equal to `word` (wordIndex).
pub fn word_index(text: &str, word: &str) -> Option<usize> {
    text.split_whitespace().position(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).eq_ignore_ascii_case(word))
}

/// A short, single-line excerpt over the first lines (excerpt), reusing the shared shortener.
pub fn excerpt(text: &str, max: usize) -> String {
    let joined = text.lines().filter(|line| !line.trim().is_empty()).take(3).collect::<Vec<_>>().join(" ");
    crate::bdots_activity::shorten(&joined, max)
}

/// Escape HTML-significant characters into numeric entities (escapeHtml).
pub fn escape_html(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '&' => out.push_str("&#38;"),
            '<' => out.push_str("&#60;"),
            '>' => out.push_str("&#62;"),
            '"' => out.push_str("&#34;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Decode named and numeric HTML entities (decodeEntities).
pub fn decode_entities(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' {
            if let Some(end) = chars[i + 1..].iter().position(|c| *c == ';') {
                let entity: String = chars[i + 1..i + 1 + end].iter().collect();
                let decoded = match entity.as_str() {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    "nbsp" => Some('\u{00A0}'),
                    _ => entity
                        .strip_prefix("#x")
                        .or_else(|| entity.strip_prefix("#X"))
                        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                        .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse::<u32>().ok()))
                        .and_then(char::from_u32),
                };
                if let Some(c) = decoded {
                    out.push(c);
                    i += end + 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Outside text without bidi overrides and stray tag characters (cleanOutside).
pub fn clean_outside(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\u{1F3F4}' {
            let mut j = i + 1;
            while j < chars.len() && ('\u{E0020}'..='\u{E007E}').contains(&chars[j]) {
                j += 1;
            }
            if j > i + 1 && chars.get(j) == Some(&'\u{E007F}') {
                for k in i..=j {
                    out.push(chars[k]);
                }
                i = j + 1;
                continue;
            }
        }
        if ('\u{202A}'..='\u{202E}').contains(&c)
            || ('\u{2066}'..='\u{2069}').contains(&c)
            || ('\u{E0000}'..='\u{E007F}').contains(&c)
        {
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Whether outside text carries characters `cleanOutside` strips (hasInvisible).
pub fn has_invisible(text: &str) -> bool {
    clean_outside(text) != text
}

/// Outside text with invisible marks removed, for plain comparisons (visibleText).
pub fn visible_text(text: &str) -> String {
    clean_outside(text)
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '\u{00AD}' | '\u{034F}' | '\u{180E}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}'
            ) && !('\u{E0000}'..='\u{E007F}').contains(c)
        })
        .collect()
}

/// Whether a part occurs in a whole after invisible marks are ignored (holdsText).
pub fn holds_text(whole: &str, part: &str) -> bool {
    visible_text(whole).to_lowercase().contains(&visible_text(part).to_lowercase())
}

/// HTML converted to readable text and sanitised (htmlToText), reusing the shared converter.
pub fn html_to_text(html: &str) -> String {
    clean_outside(&crate::bdots_extra::plain_text(html))
}

/// Upper-case the first character of a value (capitalize / upperFirst).
pub fn upper_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Lower-case the first character of a value (lowerFirst).
pub fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Keep a fact on one line by replacing spaces with non-breaking spaces (keepWhole).
pub fn keep_whole(value: &str) -> String {
    value.replace(' ', "\u{00A0}")
}

/// Semantic tone for text hygiene.
pub fn hygiene_tone(valid: usize, invalid: usize, hidden: usize) -> &'static str {
    if invalid == 0 && hidden == 0 && valid == 0 {
        "secondary"
    } else if invalid == 0 && hidden == 0 {
        "good"
    } else {
        "attention"
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(slug: &str, valid: usize, invalid: usize, hidden: usize) -> Vec<(&'static str, String)> {
    vec![
        ("slug", slug.to_owned()),
        ("emails", count_label(valid)),
        ("invalid", count_label(invalid)),
        ("hidden", count_label(hidden)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_and_handles() {
        assert_eq!(slugify("  Acme Saúde  "), "acme-saude");
        assert_eq!(slugify("A B"), "a-b");
        assert_eq!(handle_from("Íris B."), "irisb");
        assert_eq!(handle_from("A"), "a0");
        assert_eq!(handle_from(&"x".repeat(40)).len(), 32);
    }

    #[test]
    fn addresses() {
        assert!(is_email("ana@x.com"));
        assert!(!is_email("a@b"));
        assert!(!is_email("ana <ana@x.com>"));
        let found = addresses_in("Ana Silva <ana@x.com> \"bob@y.com\" mailto:bad@z ana@x.com");
        assert_eq!(found.len(), 3);
        assert!(found[0].1);
        assert!(found[1].1);
        assert!(!found[2].1);
        let (valid, invalid) = emails_from("ana@x.com, bad@z", &["ANA@X.COM".to_owned()]);
        assert!(valid.is_empty());
        assert_eq!(invalid, vec!["bad@z"]);
    }

    #[test]
    fn passwords_and_amounts() {
        assert_eq!(password_state(""), "empty");
        assert_eq!(password_state("1234567"), "short");
        assert_eq!(password_state("12345678"), "ok");
        assert_eq!(password_state(&"x".repeat(129)), "long");
        assert!(!looks_like_password("abcdefg1"));
        assert!(looks_like_password("abcdefg1!"));
        assert!(name_is_password(" Jean ", "Jean"));
        assert_eq!(parse_amount("40,5"), Some(40.5));
        assert_eq!(parse_amount("1.000"), Some(1000.0));
        assert_eq!(parse_amount("abc"), None);
    }

    #[test]
    fn sentence_word_and_excerpt() {
        assert_eq!(email_typo_fix("ana@gmial.com").as_deref(), Some("ana@gmail.com"));
        assert_eq!(email_typo_fix("ana@x.com"), None);
        assert_eq!(end_sentence("hi"), "hi.");
        assert_eq!(end_sentence("hi!"), "hi!");
        assert_eq!(around("hello world", "world"), Some(("hello ", "")));
        assert_eq!(word_index("hello brave world", "brave"), Some(1));
        assert_eq!(word_index("hello", "nope"), None);
        assert_eq!(excerpt("a\nb\nc\nd", 5), "a b c");
    }

    #[test]
    fn escaping_and_entities() {
        assert_eq!(escape_html("<a>"), "&#60;a&#62;");
        assert_eq!(decode_entities("&lt;a&gt;&amp;&#39;"), "<a>&'");
        assert_eq!(decode_entities("plain"), "plain");
        assert!(html_to_text("<p>hi</p><br>there").contains("hi"));
    }

    #[test]
    fn invisible_text() {
        assert_eq!(clean_outside("a\u{202E}b"), "ab");
        assert!(has_invisible("a\u{202E}b"));
        assert!(!has_invisible("plain text"));
        assert_eq!(visible_text("a\u{200B}b"), "ab");
        assert!(holds_text("Hello world", "\u{200B}world"));
    }

    #[test]
    fn first_letters_and_summary() {
        assert_eq!(upper_first("hello"), "Hello");
        assert_eq!(lower_first("Hello"), "hello");
        assert_eq!(upper_first(""), "");
        assert_eq!(keep_whole("a b"), "a\u{00A0}b");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(hygiene_tone(0, 0, 0), "secondary");
        assert_eq!(hygiene_tone(1, 0, 0), "good");
        assert_eq!(hygiene_tone(1, 1, 0), "attention");
        assert_eq!(summary("bot", 2, 1, 0).len(), 4);
    }
}
