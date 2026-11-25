//! The tool definitions the chat MCP server lists. Ported from `OpenBot` nightly's
//! `BROWSER_TOOL_DEFINITIONS` (`src/backend/browser-tools.ts`) with Codync's wording: an element ref
//! instead of `OpenBot`'s target kinds, no tab id, and none of the vault, card fill, takeover, image,
//! recording, upload, environment or dialog tools.

use serde_json::{Value, json};

/// What every action that changes the page answers, as `OpenBot` words it.
const ACTION_RESULT: &str = "Returns a status; pass snapshot: true to read the page afterwards.";
const NEEDS_ACCESS: &str = "The bot needs Use the computer on, with its browser permission.";
/// OpenBot 0.78.0: a page that crashes is named in the result, with the reload that recovers it.
const CRASH_NOTE: &str = " If the page crashes, the result says so and tells you to reload with navigate direction=reload, then take a new snapshot.";

pub(super) fn definitions() -> Vec<Value> {
    vec![
        navigate(),
        snapshot(),
        screenshot(),
        click(),
        type_text(),
        press(),
        hover(),
        drag(),
        scroll(),
        select_option(),
        set_checked(),
        wait_for(),
        evaluate(),
        run(),
    ]
}

fn navigate() -> Value {
    json!({
        "name": "navigate",
        "description": format!("Open an http or https address in this bot's own browser tab, or go back, forward or reload, wait for the page to finish loading, and return a snapshot of it. The first call opens the browser; the other browser tools then act on what it loaded. {NEEDS_ACCESS}"),
        "inputSchema": {"type": "object", "properties": {
            "url": {"type": "string", "description": "The address to open. https:// is added to a bare host."},
            "direction": {"type": "string", "enum": ["back", "forward", "reload"], "description": "Move in the tab's history instead of opening an address."},
            "timeoutMs": timeout("How long to wait for the page, at most 20000."),
        }, "minProperties": 1},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn snapshot() -> Value {
    json!({
        "name": "snapshot",
        "description": format!(r#"Read the page as a compact tree: one line per element with its ref, role, "name", <tag>, value and state, in page order and indented by depth, then the page's visible text. Use the refs with click, type, press, hover, scroll, select_option and set_checked. Refs stay valid until the next snapshot or a navigation. The output is bounded: 400 elements and about 40000 characters, with a note when something was left out, so scroll or wait_for to read more. A bot check adds a `challenge` field with a status and one next step: click the checkbox once when told to, otherwise ask the person to take over. Saved preferences for this site appear as up to five `siteMemories`, once per host per turn.{CRASH_NOTE}"#),
        "inputSchema": {"type": "object", "properties": {}},
        "annotations": {"readOnlyHint": true},
    })
}

fn screenshot() -> Value {
    json!({
        "name": "screenshot",
        "description": format!("Save a PNG of the page the bot's browser shows and attach it to your reply as a file card, so the user can see what you see. {NEEDS_ACCESS}"),
        "inputSchema": {"type": "object", "properties": {
            "timeoutMs": timeout("How long to wait for the picture, at most 20000."),
        }},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn click() -> Value {
    json!({
        "name": "click",
        "description": format!("Click the element whose ref the last snapshot printed, with real mouse input at its centre. {ACTION_RESULT} A click that navigates leaves the old refs behind: take a new snapshot."),
        "inputSchema": {"type": "object", "properties": {
            "ref": reference("The element's ref from the last snapshot."),
            "button": {"type": "string", "enum": ["left", "middle", "right"]},
            "clickCount": {"type": "integer", "minimum": 1, "maximum": 2, "description": "2 double-clicks."},
            "modifiers": modifiers("Keys held during the click."),
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }, "required": ["ref"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn type_text() -> Value {
    json!({
        "name": "type",
        "description": format!("Enter text with real keyboard input into the element whose ref the last snapshot printed, or into whatever the page has focused when no ref is given. mode replace (the default) replaces the field's text and append adds to the end, for a ref only; submit presses Enter afterwards. {ACTION_RESULT}"),
        "inputSchema": {"type": "object", "properties": {
            "ref": reference("The field's ref from the last snapshot. Leave it out to type into the focused control."),
            "text": {"type": "string", "description": "The text to enter, at most 20000 characters. Newlines work in a textarea."},
            "mode": {"type": "string", "enum": ["replace", "append"]},
            "submit": {"type": "boolean", "description": "Press Enter after the text."},
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }, "required": ["text"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn press() -> Value {
    json!({
        "name": "press",
        "description": format!("Press a key or a shortcut such as Enter, Tab, Escape, Control+A or Meta+Shift+P, in the page or on the element whose ref is given. {ACTION_RESULT}"),
        "inputSchema": {"type": "object", "properties": {
            "key": {"type": "string", "description": "The key, with its modifiers joined by +: Alt, Control, Meta, Shift."},
            "ref": reference("Focus this element first."),
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }, "required": ["key"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn hover() -> Value {
    json!({
        "name": "hover",
        "description": format!("Move the mouse onto the element whose ref the last snapshot printed, which opens what the page shows on hover. {ACTION_RESULT}"),
        "inputSchema": {"type": "object", "properties": {
            "ref": reference("The element's ref from the last snapshot."),
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }, "required": ["ref"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn drag() -> Value {
    json!({
        "name": "drag",
        "description": format!("Drag the element whose ref is `from` onto the one whose ref is `to`, with real press, move and release mouse input, as moving an item into a page needs. {ACTION_RESULT} The refs come from the last snapshot."),
        "inputSchema": {"type": "object", "properties": {
            "from": reference("The ref of the element to drag from."),
            "to": reference("The ref of the element to drop onto."),
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }, "required": ["from", "to"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn scroll() -> Value {
    json!({
        "name": "scroll",
        "description": format!("Scroll the page, or the element whose ref is given, by deltaX and deltaY pixels: positive scrolls right and down. {ACTION_RESULT}"),
        "inputSchema": {"type": "object", "properties": {
            "ref": reference("Scroll this element instead of the page."),
            "deltaX": {"type": "number", "description": "Pixels to scroll right, at most 100000 either way."},
            "deltaY": {"type": "number", "description": "Pixels to scroll down, at most 100000 either way."},
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn select_option() -> Value {
    json!({
        "name": "select_option",
        "description": format!("Select one or more options of the native select whose ref the last snapshot printed, by each option's value or its visible label, and tell the page the value changed. {ACTION_RESULT}"),
        "inputSchema": {"type": "object", "properties": {
            "ref": reference("The select's ref from the last snapshot."),
            "values": {"type": "array", "minItems": 1, "maxItems": 100, "items": {"type": "string", "maxLength": 1000}, "description": "The option values or labels to select."},
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }, "required": ["ref", "values"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn set_checked() -> Value {
    json!({
        "name": "set_checked",
        "description": format!("Set the checkbox or radio button whose ref the last snapshot printed to checked or unchecked, as a person's click would. {ACTION_RESULT}"),
        "inputSchema": {"type": "object", "properties": {
            "ref": reference("The checkbox or radio button's ref from the last snapshot."),
            "checked": {"type": "boolean"},
            "timeoutMs": timeout("How long to wait, at most 20000."),
            "snapshot": snapshot_flag(),
        }, "required": ["ref", "checked"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn wait_for() -> Value {
    json!({
        "name": "wait_for",
        "description": "Wait until the page shows a text, reaches an address, or finishes loading or changing, then return a fresh snapshot. Use it after a click or a submit instead of guessing how long the next page takes.",
        "inputSchema": {"type": "object", "properties": {
            "text": {"type": "string", "maxLength": 2000, "description": "Wait until this text is visible on the page."},
            "url": {"type": "string", "description": "Wait until the address contains this part."},
            "state": {"type": "string", "enum": ["load", "domcontentloaded", "dom-quiet"], "description": "dom-quiet: the page changed nothing for half a second."},
            "timeoutMs": timeout("How long to wait before giving up, at most 20000."),
        }},
        "annotations": {"readOnlyHint": true},
    })
}

fn evaluate() -> Value {
    json!({
        "name": "evaluate",
        "description": "Evaluate a JavaScript expression in the page itself, with its DOM, page scripts and cookies, and return the value as JSON, cut at 64000 characters. Prefer snapshots and the other tools; use this to read what they cannot, or for a control they cannot reach. A value that is not JSON-serializable is reported as such.",
        "inputSchema": {"type": "object", "properties": {
            "expression": {"type": "string", "description": "The JavaScript to evaluate, at most 64000 characters."},
            "awaitPromise": {"type": "boolean", "description": "Wait for the expression's promise."},
            "timeoutMs": timeout("How long to wait for the value, at most 20000."),
        }, "required": ["expression"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn run() -> Value {
    json!({
        "name": "run",
        "description": "Run a list of browser steps in one call and return a final snapshot: the same actions as the other browser tools, each with that tool's fields, at most 200 steps and one time limit. A failed step stops the run, and the steps before it stay done. This is a step list that Codync runs, not JavaScript.",
        "inputSchema": {"type": "object", "properties": {
            "steps": {"type": "array", "minItems": 1, "maxItems": 200, "description": "The steps, in order. Each names its action (navigate, screenshot, snapshot, click, type, press, hover, drag, scroll, select_option, set_checked, wait_for, evaluate or sleep) and takes the fields of that tool; a step acts on the same tab.", "items": {"type": "object", "properties": {"action": {"type": "string", "enum": ["navigate", "snapshot", "screenshot", "click", "type", "press", "hover", "drag", "scroll", "select_option", "set_checked", "wait_for", "evaluate", "sleep"]}, "ms": {"type": "integer", "description": "sleep: milliseconds to pause, at most 25000."}}, "required": ["action"]}},
            "timeoutMs": timeout("The whole run's limit, at most 25000."),
        }, "required": ["steps"]},
        "annotations": {"readOnlyHint": false, "openWorldHint": true},
    })
}

fn reference(description: &str) -> Value {
    json!({"type": "integer", "minimum": 1, "description": description})
}

fn timeout(description: &str) -> Value {
    json!({"type": "integer", "minimum": 0, "maximum": 20_000, "description": description})
}

fn snapshot_flag() -> Value {
    json!({"type": "boolean", "description": "Read the page again after the action and return its snapshot."})
}

fn modifiers(description: &str) -> Value {
    json!({
        "type": "array",
        "items": {"type": "string", "enum": ["Alt", "Control", "Meta", "Shift"]},
        "maxItems": 4,
        "description": description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_tool_has_a_name_a_description_and_an_object_schema() {
        let definitions = definitions();
        assert_eq!(definitions.len(), 14);
        let names: HashSet<&str> = definitions.iter().filter_map(|tool| tool["name"].as_str()).collect();
        assert_eq!(names.len(), definitions.len(), "each tool name appears once");
        for expected in [
            "navigate",
            "snapshot",
            "screenshot",
            "click",
            "type",
            "press",
            "hover",
            "drag",
            "scroll",
            "select_option",
            "set_checked",
            "wait_for",
            "evaluate",
            "run",
        ] {
            assert!(names.contains(expected), "{expected} is listed");
        }
        for tool in &definitions {
            assert!(tool["description"].as_str().is_some_and(|text| text.len() > 40), "{tool} describes itself");
            assert_eq!(tool["inputSchema"]["type"], "object", "{tool} takes an object");
        }
    }

    #[test]
    fn a_snapshot_names_the_crash_reload_in_its_description() {
        let snapshot = definitions().into_iter().find(|tool| tool["name"] == "snapshot").expect("snapshot is listed");
        let description = snapshot["description"].as_str().unwrap_or_default();
        assert!(description.contains("page crashes"), "{description}");
        assert!(description.contains("navigate direction=reload"), "{description}");
    }

    #[test]
    fn the_step_list_matches_the_tools_it_runs() {
        let run = definitions().into_iter().find(|tool| tool["name"] == "run").expect("run is listed");
        let actions = run["inputSchema"]["properties"]["steps"]["items"]["properties"]["action"]["enum"]
            .as_array()
            .expect("the steps name their actions");
        for tool in definitions().iter().filter(|tool| tool["name"] != "run") {
            let name = tool["name"].as_str().unwrap_or_default();
            assert!(actions.iter().any(|action| action == name), "a run step can be {name}");
        }
        assert!(actions.iter().any(|action| action == "sleep"));
    }
}
