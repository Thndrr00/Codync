#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `engine/media.ts`: sticker moods, GIF categories, media
//! metadata and settings. Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Tool used to send a GIF (SEND_GIF_TOOL).
pub const SEND_GIF_TOOL: &str = "send_gif";

/// Tool used to send a sticker (SEND_STICKER_TOOL).
pub const SEND_STICKER_TOOL: &str = "send_sticker";

/// Moods an agent may pick for a sticker (STICKER_MOODS).
pub const STICKER_MOODS: [&str; 7] = ["celebrate", "love", "deal", "thinking", "wow", "dizzy", "wave"];

/// Moods an agent is actually allowed to send (AGENT_STICKER_MOODS).
pub const AGENT_STICKER_MOODS: [&str; 4] = ["celebrate", "love", "deal", "wow"];

/// GIF ratings allowed (GIF_RATINGS).
pub const GIF_RATINGS: [&str; 2] = ["g", "pg"];

/// How often an agent may use media (AGENT_MEDIA_LEVELS).
pub const AGENT_MEDIA_LEVELS: [&str; 3] = ["never", "sometimes", "often"];

/// Metadata key of media after a reply (MEDIA_AFTER_KEY).
pub const MEDIA_AFTER_KEY: &str = "mediaAfter";

/// GIF categories and the moods they serve.
pub const GIF_CATEGORIES: [(&str, &str); 6] = [
    ("celebrate", "Congrats and wins"),
    ("love", "Affection"),
    ("deal", "Agreement"),
    ("thinking", "Thinking"),
    ("wow", "Surprise"),
    ("wave", "Greetings"),
];

/// Whether a tool sends media (MEDIA_TOOLS).
pub fn is_media_tool(name: &str) -> bool {
    name == SEND_GIF_TOOL || name == SEND_STICKER_TOOL
}

/// Whether a mood is a known sticker mood (isStickerMood).
pub fn is_sticker_mood(mood: &str) -> bool {
    STICKER_MOODS.contains(&mood)
}

/// Whether an agent may send a mood (agentMood).
pub fn agent_mood(mood: &str) -> bool {
    AGENT_STICKER_MOODS.contains(&mood)
}

/// Category ids in order (GIF_CATEGORY_IDS).
pub fn gif_category_ids() -> Vec<&'static str> {
    GIF_CATEGORIES.iter().map(|(id, _)| *id).collect()
}

/// Label of a GIF category.
pub fn gif_category_label(id: &str) -> &'static str {
    GIF_CATEGORIES.iter().find(|(candidate, _)| *candidate == id).map_or("Other", |(_, label)| *label)
}

/// Whether a rating is allowed.
pub fn is_rating_allowed(rating: &str) -> bool {
    GIF_RATINGS.contains(&rating)
}

/// Read media metadata from a message (mediaOf).
pub fn media_of(meta: Option<&Value>) -> Value {
    let raw = meta.and_then(|value| value.get("media"));
    if !raw.is_some_and(Value::is_object) {
        return Value::Null;
    }
    let kind = raw.and_then(|value| value["type"].as_str()).unwrap_or("image");
    let id = raw.and_then(|value| value["id"].as_str()).unwrap_or("");
    if id.is_empty() {
        return Value::Null;
    }
    json!({"type": kind, "id": id})
}

/// Whether media is queued until after the reply (isMediaAfterReply).
pub fn is_media_after_reply(meta: Option<&Value>) -> bool {
    meta.and_then(|value| value.get(MEDIA_AFTER_KEY)).and_then(Value::as_bool).unwrap_or(false)
}

/// Whether a message is only media (isMediaOnly).
pub fn is_media_only(message: &Value) -> bool {
    message["body"].as_str().unwrap_or("").trim().is_empty()
        && message["meta"].get("media").is_some_and(Value::is_object)
}

/// Default media settings for a workspace (DEFAULT_MEDIA_SETTINGS).
pub fn default_media_settings() -> Value {
    json!({
        "stickers": "sometimes",
        "gifs": "sometimes",
        "rating": "g",
        "categories": ["celebrate", "love", "deal", "wave"],
    })
}

/// Normalize saved media settings (mediaSettingsFrom).
pub fn media_settings_from(saved: Option<&Value>) -> Value {
    let mut out = default_media_settings();
    let Some(saved) = saved.filter(|value| value.is_object()) else {
        return out;
    };
    for key in ["stickers", "gifs"] {
        if let Some(value) = saved[key].as_str().filter(|value| AGENT_MEDIA_LEVELS.contains(value)) {
            out[key] = Value::String(value.to_owned());
        }
    }
    if let Some(rating) = saved["rating"].as_str().filter(|value| is_rating_allowed(value)) {
        out["rating"] = Value::String(rating.to_owned());
    }
    let categories = gif_category_ids();
    if let Some(values) = saved["categories"].as_array() {
        let chosen: Vec<String> = values
            .iter()
            .filter_map(Value::as_str)
            .filter(|value| categories.contains(value))
            .map(str::to_owned)
            .collect();
        if !chosen.is_empty() {
            out["categories"] = json!(chosen);
        }
    }
    out
}

/// Cache key of a workspace's media settings (mediaSettingsCacheKey).
pub fn media_settings_cache_key(org_id: &str) -> String {
    format!("{org_id}:media")
}

/// Whether media is allowed at a level.
pub fn media_allowed(level: &str) -> bool {
    level == "sometimes" || level == "often"
}

/// One line describing a media message (mediaLine).
pub fn media_line(kind: &str, id: &str, after: bool) -> String {
    format!("{kind} · {id} · {}", if after { "after the reply" } else { "with the reply" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(moods: usize, categories: usize, stickers: bool, gifs: bool) -> Vec<(&'static str, String)> {
    vec![
        ("moods", count_label(moods)),
        ("categories", count_label(categories)),
        ("stickers", if stickers { "yes" } else { "no" }.to_owned()),
        ("gifs", if gifs { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_and_moods() {
        assert!(is_media_tool("send_gif"));
        assert!(!is_media_tool("read_file"));
        assert!(is_sticker_mood("celebrate"));
        assert!(!is_sticker_mood("unknown"));
        assert!(agent_mood("love"));
        assert!(!agent_mood("thinking"));
        assert_eq!(gif_category_ids().len(), 6);
        assert_eq!(gif_category_label("wow"), "Surprise");
        assert!(is_rating_allowed("pg"));
        assert!(!is_rating_allowed("r"));
        assert_eq!(STICKER_MOODS.len(), 7);
        assert_eq!(AGENT_MEDIA_LEVELS[2], "often");
    }

    #[test]
    fn media_and_settings() {
        let meta = json!({"media":{"type":"image","id":"m1"},"mediaAfter":true});
        assert_eq!(media_of(Some(&meta))["id"], "m1");
        assert_eq!(media_of(None), Value::Null);
        assert!(is_media_after_reply(Some(&meta)));
        assert!(is_media_only(&json!({"body":"","meta":meta})));
        assert!(!is_media_only(&json!({"body":"hi","meta":{}})));
        let settings = default_media_settings();
        assert_eq!(settings["stickers"], "sometimes");
        assert_eq!(media_settings_cache_key("org1"), "org1:media");
        assert!(media_allowed("often"));
        assert!(!media_allowed("never"));
    }

    #[test]
    fn normalization_and_presentation() {
        let saved = json!({"stickers":"often","gifs":"never","rating":"bad","categories":["love","bogus"]});
        let normalized = media_settings_from(Some(&saved));
        assert_eq!(normalized["stickers"], "often");
        assert_eq!(normalized["rating"], "g");
        assert_eq!(normalized["categories"].as_array().map_or(0, Vec::len), 1);
        assert_eq!(media_settings_from(None)["gifs"], "sometimes");
        assert_eq!(media_line("image", "m1", true), "image · m1 · after the reply");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(7, 6, true, true).len(), 4);
    }
}
