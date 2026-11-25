//! How a page's snapshot reads to the model: the address and title, one line per element the page
//! offers (indented by depth, with the ref in front), then the page's own visible text, cut to fit.
//! Ported from `OpenBot` nightly's snapshot text (`src/backend/browser-cdp.ts`): ref, role, `"name"`,
//! `<tag>`, then the value and states when an element has them.

use super::cut;
use serde_json::Value;

/// The most a snapshot may take, in characters.
pub(super) const MAX_CHARS: usize = 40_000;
const MAX_NAME: usize = 120;
const MAX_VALUE: usize = 80;
const MAX_TEXT: usize = 4_000;
/// The deepest indentation, so a long chain of wrappers stays a tree and not a wall.
const MAX_DEPTH: usize = 8;
const INDENT: &str = "  ";

/// The text of one snapshot, from what `page.js` returned.
pub(super) fn format(page: &Value, max: usize) -> String {
    let mut text = String::new();
    text.push_str("url: ");
    text.push_str(page["url"].as_str().unwrap_or("about:blank"));
    text.push('\n');
    let title = one_line(page["title"].as_str().unwrap_or_default(), MAX_NAME * 2);
    if !title.is_empty() {
        text.push_str("title: ");
        text.push_str(&title);
        text.push('\n');
    }
    let elements: &[Value] = page["elements"].as_array().map_or(&[], Vec::as_slice);
    if elements.is_empty() {
        text.push_str("(no elements)\n");
    }
    for element in elements {
        let depth = usize::try_from(element["depth"].as_u64().unwrap_or(0)).unwrap_or(0).min(MAX_DEPTH);
        text.push_str(&INDENT.repeat(depth));
        text.push_str(&element["ref"].as_u64().unwrap_or(0).to_string());
        text.push(' ');
        text.push_str(element["role"].as_str().unwrap_or("element"));
        let name = one_line(element["name"].as_str().unwrap_or_default(), MAX_NAME);
        if !name.is_empty() {
            text.push_str(" \"");
            text.push_str(&name);
            text.push('"');
        }
        text.push_str(" <");
        text.push_str(element["tag"].as_str().unwrap_or("div"));
        text.push('>');
        let value = one_line(element["value"].as_str().unwrap_or_default(), MAX_VALUE);
        if !value.is_empty() {
            text.push_str(" value=\"");
            text.push_str(&value);
            text.push('"');
        }
        for state in element["states"].as_array().into_iter().flatten() {
            if let Some(state) = state.as_str() {
                text.push(' ');
                text.push_str(state);
            }
        }
        text.push('\n');
    }
    let body = one_line(page["text"].as_str().unwrap_or_default(), MAX_TEXT);
    if !body.is_empty() {
        text.push_str("text: ");
        text.push_str(&body);
        text.push('\n');
    }
    // Site memories are hints the user saved for this host; they read as data, not instructions.
    if let Some(memories) = page["siteMemories"].as_array() {
        for memory in memories.iter().take(5) {
            text.push_str("siteMemory: ");
            text.push_str(&one_line(&crate::chat::memory::site::line(memory), MAX_NAME * 3));
            text.push('\n');
        }
    }
    if let Some(challenge) = page["challenge"].as_object() {
        text.push_str("challenge: ");
        text.push_str(challenge["status"].as_str().unwrap_or("none"));
        let next = one_line(challenge["next"].as_str().unwrap_or_default(), MAX_TEXT);
        if !next.is_empty() {
            text.push_str(" — ");
            text.push_str(&next);
        }
        text.push('\n');
    }
    if page["truncated"] == true {
        text.push_str("(more elements were left out; scroll, or open less of the page)\n");
    }
    cut(&text, max)
}

/// One line of text: whitespace collapsed, a quote that would end the field replaced, and the whole
/// line cut to `max` characters.
fn one_line(value: &str, max: usize) -> String {
    let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
    cut(&text.replace('"', "'"), max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_snapshot_reads_as_a_numbered_tree_then_the_page_text() {
        let page = json!({
            "url": "https://example.com/login",
            "title": "Sign in",
            "elements": [
                {
                    "ref": 1,
                    "depth": 0,
                    "role": "textbox",
                    "name": "Email",
                    "tag": "input",
                    "value": "ada@example.com",
                    "states": []
                },
                {
                    "ref": 2,
                    "depth": 1,
                    "role": "checkbox",
                    "name": "Remember \"me\"",
                    "tag": "input",
                    "value": "",
                    "states": ["checked"]
                },
                {
                    "ref": 3,
                    "depth": 12,
                    "role": "button",
                    "name": "Sign\nin",
                    "tag": "button",
                    "value": "",
                    "states": ["disabled"]
                }
            ],
            "text": "Welcome\n back",
            "truncated": false
        });
        let expected = format!(
            "url: https://example.com/login\n\
             title: Sign in\n\
             1 textbox \"Email\" <input> value=\"ada@example.com\"\n\
             2 checkbox \"Remember 'me'\" <input> checked\n\
             {indent}3 button \"Sign in\" <button> disabled\n\
             text: Welcome back\n",
            indent = INDENT.repeat(MAX_DEPTH)
        );
        assert_eq!(format(&page, MAX_CHARS), expected);
    }

    #[test]
    fn an_empty_page_says_it_has_no_elements() {
        let empty = json!({"url": "about:blank", "title": "", "elements": [], "truncated": false});
        assert_eq!(format(&empty, MAX_CHARS), "url: about:blank\n(no elements)\n");
    }

    #[test]
    fn a_cut_snapshot_says_where_it_stopped() {
        let long = json!({
            "url": "https://example.com",
            "elements": [{"ref": 1, "role": "button", "name": "x".repeat(500), "tag": "button"}],
            "truncated": true
        });
        let text = format(&long, 120);
        assert!(text.len() <= 160, "the snapshot stays inside its limit: {text}");
        assert!(text.contains("cut at 120 characters"), "a cut snapshot says where it stopped: {text}");
    }

    #[test]
    fn saved_site_hints_read_as_bounded_data_after_the_page() {
        let page = json!({
            "url": "https://shop.example.com",
            "elements": [],
            "siteMemories": [
                {"site": "example.com", "text": "Prefers concise answers"},
                {"site": "shop.example.com", "text": "The invoice page is slow"}
            ],
            "truncated": false
        });
        let text = format(&page, MAX_CHARS);
        assert!(text.contains("siteMemory: site: example.com — Prefers concise answers"), "{text}");
        assert!(text.contains("siteMemory: site: shop.example.com — The invoice page is slow"), "{text}");
        let long = json!({"url": "https://example.com", "elements": [],
            "siteMemories": (0..20).map(|n| json!({"site": "example.com", "text": format!("hint {n}")})).collect::<Vec<_>>()});
        let capped = format(&long, MAX_CHARS);
        assert_eq!(capped.matches("siteMemory:").count(), 5, "at most five hints are printed");
    }

    #[test]
    fn a_bot_check_adds_one_bounded_line_with_its_next_step() {
        let page = json!({
            "url": "https://example.com",
            "elements": [],
            "challenge": {"status": "checkbox", "next": "Click the checkbox once, then take a new snapshot."},
            "truncated": false
        });
        let text = format(&page, MAX_CHARS);
        assert!(text.contains("challenge: checkbox — Click the checkbox once"), "{text}");
        assert!(text.ends_with('\n'), "{text}");
        let no_check = json!({"url": "https://example.com", "elements": [], "truncated": false});
        assert!(!format(&no_check, MAX_CHARS).contains("challenge:"));
    }

    #[test]
    fn the_truncation_note_appears_when_elements_were_left_out() {
        let page = json!({"url": "https://example.com", "elements": [], "truncated": true});
        assert!(format(&page, MAX_CHARS).contains("more elements were left out"));
    }
}
