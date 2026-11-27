//! Every message records how it arrived; only the person's own words teach or grant (ADR 0250).
//!
//! Codync already tags entries with `source`; this is the vocabulary that separates words typed in
//! our apps from anything relayed (a channel, voice, a routine, another bot or a connector). Only
//! `typed` counts as the person's own when learning preferences or granting something.

use serde_json::{Value, json};

/// Origins a message can have. `typed` is the only one that is the person's own words.
pub const ORIGINS: [&str; 6] = ["typed", "relayed", "voice", "routine", "agent", "connector"];

pub fn origin(saved: Option<&str>) -> Value {
    let o = saved.filter(|v| ORIGINS.contains(v)).unwrap_or("typed");
    json!({"origin": o, "ownWords": o == "typed"})
}

/// A relayed person message is theirs to answer, never theirs to teach from.
pub fn relayed_person_message(from: &str) -> Value {
    json!({"origin": "relayed", "from": from, "ownWords": false, "answerable": true})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_typed_words_are_the_persons_own() {
        assert_eq!(origin(Some("typed"))["ownWords"], true);
        assert_eq!(origin(Some("relayed"))["ownWords"], false);
        assert_eq!(origin(Some("routine"))["ownWords"], false);
        assert_eq!(origin(Some("bogus"))["origin"], "typed");
        assert_eq!(relayed_person_message("slack")["answerable"], true);
        assert_eq!(relayed_person_message("slack")["ownWords"], false);
    }
}
