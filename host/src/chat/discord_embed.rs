//! An agent posts a Discord embed card (OpenBot 0.66.0: "Agents can post Discord embeds"): a title, a
//! description, fields and an image in the message, instead of plain text. The bot must be connected to
//! Discord; the card goes straight to the channel the model names.

use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

pub const TOOL: &str = "post_discord_embed";
/// Discord's own limits for an embed.
const MAX_TITLE: usize = 256;
const MAX_DESCRIPTION: usize = 4096;
const MAX_FIELD_VALUE: usize = 1024;
const MAX_FOOTER: usize = 2048;
const MAX_FIELDS: usize = 25;

pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Post a Discord embed card to a channel the bot can write to: a title, a description, a link, a colour, up to 25 fields, a footer, an author line and an image URL. Use it when a card reads better than a wall of text; plain messages still go as text.",
        "inputSchema": {"type":"object","properties":{
            "channel": {"type":"string","description":"The Discord channel id to post in."},
            "title": {"type":"string","description":"At most 256 characters."},
            "description": {"type":"string","description":"At most 4096 characters."},
            "url": {"type":"string","description":"A link the title opens."},
            "color": {"type":"integer","description":"0xRRGGBB, such as 3447003."},
            "fields": {"type":"array","description":"Up to 25, each with name, value and whether it sits inline.","items":{"type":"object","properties":{"name":{"type":"string"},"value":{"type":"string"},"inline":{"type":"boolean"}},"required":["name","value"]}},
            "footer": {"type":"string"},
            "image": {"type":"string","description":"An http(s) image URL."},
            "thumbnail": {"type":"string","description":"An http(s) image URL."},
            "author": {"type":"string"}
        },"required":["channel","title"]},
        "annotations": {"readOnlyHint": false},
    })
}

fn http(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

/// The embed Discord accepts, from what the model asked for, cut to Discord's own limits.
pub fn embed_of(args: &Value) -> Result<Value> {
    let text = |key: &str| args[key].as_str().map(str::trim).filter(|value| !value.is_empty());
    let cut = |value: &str, max: usize| -> String { value.chars().take(max).collect() };
    let Some(title) = text("title") else { bail!("A Discord card needs a title.") };
    let mut embed = json!({"title": cut(title, MAX_TITLE)});
    if let Some(description) = text("description") {
        embed["description"] = json!(cut(description, MAX_DESCRIPTION));
    }
    if let Some(url) = text("url").filter(|url| http(url)) {
        embed["url"] = json!(url);
    }
    if let Some(color) = args["color"].as_u64().filter(|color| *color <= 0xFF_FF_FF) {
        embed["color"] = json!(color);
    }
    if let Some(fields) = args["fields"].as_array() {
        let fields: Vec<Value> = fields
            .iter()
            .take(MAX_FIELDS)
            .filter_map(|field| {
                let name = field["name"].as_str()?.trim();
                let value = field["value"].as_str()?.trim();
                (!name.is_empty() && !value.is_empty()).then(|| {
                    json!({
                        "name": cut(name, MAX_TITLE),
                        "value": cut(value, MAX_FIELD_VALUE),
                        "inline": field["inline"] == true,
                    })
                })
            })
            .collect();
        if !fields.is_empty() {
            embed["fields"] = json!(fields);
        }
    }
    if let Some(footer) = text("footer") {
        embed["footer"] = json!({"text": cut(footer, MAX_FOOTER)});
    }
    if let Some(image) = text("image").filter(|url| http(url)) {
        embed["image"] = json!({"url": image});
    }
    if let Some(thumbnail) = text("thumbnail").filter(|url| http(url)) {
        embed["thumbnail"] = json!({"url": thumbnail});
    }
    if let Some(author) = text("author") {
        embed["author"] = json!({"name": cut(author, MAX_TITLE)});
    }
    Ok(embed)
}

pub async fn call(hub: &Arc<Hub>, _bot: &str, args: &Value) -> Result<Value> {
    let channel =
        args["channel"].as_str().map(str::trim).filter(|channel| !channel.is_empty()).context("channel is required")?;
    let embed = embed_of(args)?;
    crate::discord::send_embed(&hub.store, channel, &embed).await?;
    Ok(json!({"posted": true, "channel": channel, "title": embed["title"]}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_is_cut_to_discords_limit_and_keeps_its_fields_in_order() {
        let long = "x".repeat(500);
        let embed = embed_of(&json!({
            "channel": "123",
            "title": long,
            "description": "A summary",
            "color": 3447003,
            "fields": [{"name": "First", "value": "one", "inline": true}, {"name": "", "value": "dropped"}, {"name": "Second", "value": "two"}],
            "footer": "Codync",
            "author": "A bot",
            "image": "https://example.com/a.png",
            "thumbnail": "not a link",
        }))
        .unwrap();
        assert_eq!(embed["title"].as_str().unwrap().chars().count(), MAX_TITLE);
        assert_eq!(embed["fields"].as_array().unwrap().len(), 2, "an empty field is left out");
        assert_eq!(embed["fields"][0]["inline"], true);
        assert_eq!(embed["footer"]["text"], "Codync");
        assert_eq!(embed["image"]["url"], "https://example.com/a.png");
        assert!(embed.get("thumbnail").is_none(), "only http(s) links");
        assert_eq!(embed["color"], 3447003);
    }

    #[test]
    fn a_card_needs_a_title_and_a_sane_colour() {
        assert!(embed_of(&json!({"channel": "1"})).is_err());
        let embed = embed_of(&json!({"title": "Hi", "color": 0x1_000_000})).unwrap();
        assert!(embed.get("color").is_none(), "a colour past 0xFFFFFF is left out");
    }
}
