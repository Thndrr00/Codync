//! Reading direction: which paragraphs of a message read right to left (Arabic, Hebrew and their
//! presentation forms), so the terminal can align them that way. Code, commands and paths always
//! read left to right.

use ratatui::layout::Alignment;

/// The right-to-left blocks of Unicode, strongest first: Hebrew, Arabic and its supplements,
/// Syriac, Thaana, NKo, Samaritan, Mandaic, and the Arabic presentation forms.
const RIGHT_TO_LEFT: [(u32, u32); 12] = [
    (0x0590, 0x05FF),
    (0x0600, 0x06FF),
    (0x0700, 0x074F),
    (0x0750, 0x077F),
    (0x0780, 0x07BF),
    (0x07C0, 0x07FF),
    (0x0800, 0x083F),
    (0x0840, 0x085F),
    (0x08A0, 0x08FF),
    (0xFB1D, 0xFB4F),
    (0xFB50, 0xFDFF),
    (0xFE70, 0xFEFF),
];

/// Whether `text` reads right to left, from its first strong character. Digits, punctuation and
/// emoji don't decide; a paragraph with no strong letter reads left to right.
pub fn reads_right_to_left(text: &str) -> bool {
    for c in text.chars() {
        let value = c as u32;
        if RIGHT_TO_LEFT.iter().any(|&(from, to)| (from..=to).contains(&value)) {
            return true;
        }
        if c.is_alphabetic() {
            return false;
        }
    }
    false
}

/// The alignment of a paragraph: right for Hebrew or Arabic text, left otherwise.
pub fn alignment(text: &str) -> Alignment {
    if reads_right_to_left(text) { Alignment::Right } else { Alignment::Left }
}

/// The paragraphs of a message, split on blank lines. A fenced code block stays one paragraph,
/// so its own blank lines don't split it.
pub fn paragraphs(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut in_code = false;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
        }
        if !in_code && trimmed.is_empty() {
            if let Some(from) = start.take() {
                out.push(&text[from..offset]);
            }
        } else if start.is_none() {
            start = Some(offset);
        }
        offset += line.len();
    }
    if let Some(from) = start {
        out.push(&text[from..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hebrew_and_arabic_read_right_to_left() {
        assert!(reads_right_to_left("שלום עולם"));
        assert!(reads_right_to_left("مرحبا بالعالم"));
        assert!(reads_right_to_left("123 שלום"));
        assert!(reads_right_to_left("«שלום»"));
        // Presentation forms, as a terminal may receive them.
        assert!(reads_right_to_left("\u{FE91}\u{FE92}"));
    }

    #[test]
    fn latin_code_and_punctuation_read_left_to_right() {
        assert!(!reads_right_to_left("hello"));
        assert!(!reads_right_to_left("12345 !!!"));
        assert!(!reads_right_to_left(""));
        assert!(!reads_right_to_left("`cargo test`"));
        assert_eq!(alignment("hello"), Alignment::Left);
        assert_eq!(alignment("שלום"), Alignment::Right);
    }

    #[test]
    fn paragraphs_split_on_blank_lines_and_keep_code_together() {
        let text = "First.\n\nSecond.\n\n```\na\n\nb\n```\n\nThird.";
        assert_eq!(paragraphs(text), ["First.\n\n", "Second.\n\n", "```\na\n\nb\n```\n\n", "Third."]);
        assert_eq!(paragraphs("  \n\nOne."), ["One."]);
        assert!(paragraphs("").is_empty());
    }
}
