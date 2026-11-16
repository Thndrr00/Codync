//! How to connect an app, as the terminal draws it above a form: the host's steps and the vendor's
//! page. Shared by the app sheet and Composio's.

use serde_json::Value;

/// The guide's steps and link, one per line, ready to go above a sheet's fields.
fn steps_note(guide: &Value) -> Option<String> {
    let steps: Vec<&str> = guide["steps"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    let url = guide["url"].as_str();
    if steps.is_empty() && url.is_none() {
        return None;
    }
    let mut out = String::new();
    for (i, step) in steps.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str("• ");
        out.push_str(step);
    }
    if let Some(url) = url {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(url);
    }
    Some(out)
}

/// Appends `guide`'s note to `note`, when the app carries one.
pub(super) fn append(note: &mut String, guide: &Value) {
    if let Some(extra) = steps_note(guide) {
        if !note.is_empty() {
            note.push_str("\n\n");
        }
        note.push_str(&extra);
    }
}
