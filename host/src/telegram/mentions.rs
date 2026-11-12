//! Telegram entity ranges use UTF-16 offsets, including both units of characters such as emoji.

use serde_json::Value;

pub(super) fn mentions_bot(message: &Value, username: &str) -> bool {
    let Some(text) = message["text"].as_str() else { return false };
    let Some(entities) = message["entities"].as_array() else { return false };
    if username.is_empty() {
        return false;
    }
    let expected = format!("@{username}");
    let expected_units = expected.encode_utf16().count();
    entities.iter().any(|entity| {
        if entity["type"] != "mention" {
            return false;
        }
        let Some(offset) = entity["offset"].as_u64().and_then(|value| usize::try_from(value).ok()) else {
            return false;
        };
        let Some(length) = entity["length"].as_u64().and_then(|value| usize::try_from(value).ok()) else {
            return false;
        };
        // Bound allocation to the configured username, never an untrusted entity length.
        if length != expected_units || offset.checked_add(length).is_none() {
            return false;
        }
        let units: Vec<u16> = text.encode_utf16().skip(offset).take(length).collect();
        units.len() == length && String::from_utf16(&units).is_ok_and(|mention| mention.eq_ignore_ascii_case(&expected))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_mention_entities_match_ignoring_username_case() {
        assert!(mentions_bot(
            &json!({"text":"@HelperBot help", "entities":[{"type":"mention","offset":0,"length":10}]}),
            "helperbot"
        ));
    }

    #[test]
    fn another_username_with_the_same_prefix_is_not_a_mention() {
        assert!(!mentions_bot(
            &json!({"text":"@helperbot_other help", "entities":[{"type":"mention","offset":0,"length":16}]}),
            "helperbot"
        ));
    }

    #[test]
    fn emoji_before_a_mention_uses_utf16_offsets() {
        assert!(mentions_bot(
            &json!({"text":"😀 @helperbot", "entities":[{"type":"mention","offset":3,"length":10}]}),
            "helperbot"
        ));
    }

    #[test]
    fn missing_or_malformed_entity_ranges_do_not_match() {
        for entity in [
            json!({"type":"mention","offset":-1,"length":10}),
            json!({"type":"mention","offset":100,"length":10}),
            json!({"type":"mention","offset":0,"length":999_999_999_999_u64}),
            json!({"type":"mention","offset":18_446_744_073_709_551_615_u64,"length":10}),
            json!({"type":"mention","offset":0}),
            json!({"type":"text_mention","offset":0,"length":10}),
        ] {
            assert!(!mentions_bot(&json!({"text":"@helperbot", "entities":[entity]}), "helperbot"));
        }
        assert!(!mentions_bot(&json!({"text":"@helperbot"}), "helperbot"));
        assert!(!mentions_bot(&Value::Null, "helperbot"));
        assert!(!mentions_bot(
            &json!({"text":"😀 @helperbot", "entities":[{"type":"mention","offset":1,"length":10}]}),
            "helperbot"
        ));
    }
}
