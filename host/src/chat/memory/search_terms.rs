//! Query terms for saved-memory search, and which lines they really match (OpenBot nightly `triage-14438`).
//! An index that keeps a Latin word together with the unspaced text next to it misses "LP" in
//! "制作LPを確認した". The broad scan finds it; these rules keep it from also finding "HELPを読む" or a
//! part of "concatを使う" when the query is "cat".

/// Scripts written without spaces between words, where a Latin word or number can touch its neighbour.
pub(crate) fn unspaced(c: char) -> bool {
    matches!(
        c as u32,
        0x4E00..=0x9FFF // Han
            | 0x3040..=0x309F // Hiragana
            | 0x30A0..=0x30FF // Katakana
            | 0xAC00..=0xD7AF // Hangul
            | 0x0E00..=0x0E7F // Thai and Lao
            | 0x1780..=0x17FF // Khmer
            | 0x1000..=0x109F // Myanmar
    )
}

/// The words of a query, in order, capped so one call can't scan the whole memory forever.
pub(crate) fn query_terms(query: &str, limit: usize) -> Vec<String> {
    query.split_whitespace().take(limit).map(str::to_owned).collect()
}

/// Whether `term` is really in `text`. A term in an unspaced script is a plain substring. A Latin or
/// number term must be a whole word on both sides: a space, punctuation, an edge, or unspaced text.
pub(crate) fn term_matches(term: &str, text: &str) -> bool {
    let needle = term.to_lowercase();
    if needle.is_empty() {
        return true;
    }
    let lower = text.to_lowercase();
    if term.chars().any(unspaced) {
        return lower.contains(&needle);
    }
    let chars: Vec<char> = lower.chars().collect();
    let term: Vec<char> = needle.chars().collect();
    if term.len() > chars.len() {
        return false;
    }
    (0..=chars.len() - term.len()).any(|start| {
        if chars[start..start + term.len()] != term[..] {
            return false;
        }
        let left = start.checked_sub(1).and_then(|i| chars.get(i));
        let right = chars.get(start + term.len());
        let boundary = |c: &char| !c.is_alphanumeric() || unspaced(*c);
        left.is_none_or(boundary) && right.is_none_or(boundary)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_latin_word_touching_unspaced_text_is_found() {
        assert!(term_matches("LP", "制作LPを確認した"), "the reported case");
        assert!(term_matches("3D", "制作3Dプリントを出力"), "numbers touch too");
        assert!(term_matches("LP", "LPを確認した"), "start of the line");
        assert!(term_matches("LP", "制作LP"), "end of the line");
        assert!(term_matches("制作", "制作LPを確認した"), "an unspaced term stays a substring");
    }

    #[test]
    fn a_latin_word_does_not_match_part_of_a_longer_word() {
        assert!(!term_matches("LP", "HELPを読む"), "the left side is a letter");
        assert!(!term_matches("LP", "を読むLPX"), "the right side is a letter");
        assert!(!term_matches("cat", "concatを使う"));
        assert!(!term_matches("cat", "concatenate"));
    }

    #[test]
    fn a_whole_latin_word_with_spaces_still_matches() {
        assert!(term_matches("cat", "we saw a cat yesterday"));
        assert!(term_matches("cat", "a cat, then a dog"));
        assert!(term_matches("CAT", "we saw a cat"));
    }

    #[test]
    fn a_query_is_split_into_bounded_terms() {
        assert_eq!(query_terms("  LP  猫 ", 8), ["LP", "猫"]);
        assert_eq!(query_terms("a b c", 2), ["a", "b"]);
    }
}
