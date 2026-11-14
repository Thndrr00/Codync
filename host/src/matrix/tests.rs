//! The Matrix channel’s cross-cutting check: one approval becomes the card Matrix shows, and both
//! ways of answering it (a numbered reply or a reaction) name the same option.

use super::{client, render};
use crate::channels::approval::Approval;
use serde_json::json;

#[test]
fn one_approval_becomes_a_numbered_card_with_matching_reactions() {
    let entry = json!({
        "id": "e1",
        "kind": "permission",
        "data": {
            "title": "Run a command",
            "toolKind": "execute",
            "command": "cargo test",
            "detail": "in the workspace",
            "status": "pending",
            "options": [
                {"optionId": "yes", "name": "Allow"},
                {"optionId": "always", "name": "Always allow"},
                {"optionId": "no", "name": "Reject"},
            ],
        },
    });
    let approval = Approval::from_entry(&entry).expect("an approval");
    let blocks = approval.blocks(|_| None);
    let content = render::card(&blocks);
    let body = content["body"].as_str().expect("a body");
    assert!(body.contains("1) Allow"), "{body}");
    assert!(body.contains("2) Always allow"), "{body}");
    assert!(body.contains("3) Reject"), "{body}");
    // The same three positions the reply numbers name, as reaction keys.
    assert_eq!(render::reaction_index("1"), Some(0));
    assert_eq!(render::reaction_index("3️⃣"), Some(2));
    assert_eq!(approval.settled(Some("always"), "answered"), "Run a command\n✓ Always allow");
}

#[test]
fn a_settled_card_replaces_the_approval_with_its_answer() {
    let entry = json!({
        "id": "e2",
        "kind": "permission",
        "data": {"title": "Deploy", "status": "answered", "selected": "no",
            "options": [{"optionId": "yes", "name": "Allow"}, {"optionId": "no", "name": "Reject"}]},
    });
    let approval = Approval::from_entry(&entry).expect("an approval");
    let text = approval.settled(entry["data"]["selected"].as_str(), "answered");
    assert_eq!(text, "Deploy\n✓ Reject");
    // A replacement keeps the same shape a client edits into the card event.
    let edit = json!({"msgtype": "m.text", "body": format!("* {text}"),
        "m.new_content": {"msgtype": "m.text", "body": text}});
    assert_eq!(edit["m.new_content"]["body"], "Deploy\n✓ Reject");
    assert_eq!(client::text_content("Allow", "$card")["m.relates_to"]["event_id"], json!("$card"));
}
