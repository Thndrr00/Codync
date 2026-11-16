//! An approval card drawn for Matrix: the K5 blocks as plain text, plus the HTML body Matrix clients
//! render. Buttons become numbered options, answered by a reply or by a reaction the sender adds.

use crate::channels::render::{Block, Tone, fallback};
use serde_json::{Value, json};

/// The reaction key that answers option `index` (0-based).
pub(super) const KEYS: [&str; 5] = ["1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣"];

/// The `m.room.message` content of a card. `body` is what every client can read; `formatted_body`
/// is the same card with bold labels and an ordered list of options.
pub(super) fn card(blocks: &[Block]) -> Value {
    json!({
        "msgtype": "m.text",
        "body": fallback(blocks),
        "format": "org.matrix.custom.html",
        "formatted_body": html(blocks),
    })
}

fn html(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            Block::Text(text) => out.push_str(&format!("<p>{}</p>", escape(text))),
            Block::Fields(fields) => {
                for field in fields {
                    out.push_str(&format!("<p><b>{}</b>: {}</p>", escape(&field.label), escape(&field.value)));
                }
            }
            Block::Table { columns, rows } => {
                let mut lines = vec![columns.join(" | ")];
                lines.extend(rows.iter().map(|row| row.join(" | ")));
                out.push_str(&format!("<pre>{}</pre>", escape(&lines.join("\n"))));
            }
            Block::Divider => out.push_str("<hr>"),
            Block::Callout { tone, text } => {
                let mark = match tone {
                    Tone::Info => "ℹ️",
                    Tone::Success => "✅",
                    Tone::Warning => "⚠️",
                    Tone::Danger => "⛔",
                };
                out.push_str(&format!("<p>{mark} {}</p>", escape(text)));
            }
            Block::Buttons(buttons) if !buttons.is_empty() => {
                out.push_str("<ol>");
                for button in buttons {
                    out.push_str(&format!("<li>{}</li>", escape(&button.label)));
                }
                out.push_str("</ol>");
            }
            Block::Buttons(_) => {}
        }
    }
    out
}

/// Matrix's HTML body subset escapes at least these three.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The option a reaction key asks for: its number, or the key of that position.
pub(super) fn reaction_index(key: &str) -> Option<usize> {
    if let Some(index) = KEYS.iter().position(|candidate| *candidate == key) {
        return Some(index);
    }
    key.trim().parse::<usize>().ok()?.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::render::{Button, Field};

    fn blocks() -> Vec<Block> {
        vec![
            Block::Text("Needs you: Run a command".into()),
            Block::Callout { tone: Tone::Warning, text: "cargo test".into() },
            Block::Fields(vec![Field::new("Tool", "execute"), Field::new("Details", "1 file <2>")]),
            Block::Buttons(vec![Button::new("Allow", None), Button::new("Reject", None)]),
        ]
    }

    #[test]
    fn a_card_is_text_and_an_ordered_html_list() {
        let content = card(&blocks());
        assert_eq!(
            content["body"],
            "Needs you: Run a command\n⚠️ cargo test\nTool: execute\nDetails: 1 file <2>\n1) Allow\n2) Reject"
        );
        let html = content["formatted_body"].as_str().expect("html");
        assert!(html.contains("<b>Details</b>: 1 file &lt;2&gt;"), "{html}");
        assert!(html.ends_with("<ol><li>Allow</li><li>Reject</li></ol>"), "{html}");
        assert_eq!(content["format"], "org.matrix.custom.html");
    }

    #[test]
    fn a_reaction_key_finds_its_option() {
        assert_eq!(reaction_index("1️⃣"), Some(0));
        assert_eq!(reaction_index("3"), Some(2));
        assert_eq!(reaction_index("0"), None);
        assert_eq!(reaction_index("no"), None);
        assert!(KEYS.len() >= 4);
    }
}
