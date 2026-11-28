#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/mail.ts`: the Mail pill's ids, senders,
//! filters, rules and plan fences. Only `botInsights` calls this.

use serde_json::Value;

/// Integration id of the Mail pill (MAIL_ID).
pub const MAIL_ID: &str = "integration_mail";

/// Mail's blue (MAIL_BLUE).
pub const MAIL_BLUE: &str = "#2F8CFF";

/// Filters of the message list (MAIL_FILTERS).
pub const MAIL_FILTERS: [&str; 4] = ["all", "unread", "flagged", "today"];

/// Fields a rule may match (MAIL_FIELDS).
pub const MAIL_FIELDS: [&str; 5] = ["sender", "domain", "subject", "text", "account"];

/// Action kinds of a mail rule.
pub const ACTION_KINDS: [&str; 4] = ["junk", "important", "markRead", "move"];

/// Reason kinds of a triage decision.
pub const REASON_KINDS: [&str; 4] = ["vip", "banned", "rule", "manual"];

/// Fence that wraps a mail plan in an answer.
pub const PLAN_FENCE: &str = "```coucou-mail";

/// Id of a message: store and entry (messageId).
pub fn message_id(message: &Value) -> String {
    format!("{}/{}", message["storeId"].as_str().unwrap_or(""), message["entryId"].as_str().unwrap_or(""))
}

/// Sender name from the message, else the address' local part (senderName).
pub fn sender_name(message: &Value) -> &str {
    let name = message["senderName"].as_str().unwrap_or("").trim();
    if !name.is_empty() {
        return name;
    }
    let address = message["address"].as_str().unwrap_or("");
    address.split('@').next().unwrap_or(address)
}

/// Name from "Name <address>" (nameOf).
pub fn name_of(sender: &str) -> String {
    let head = sender.split_once('<').map_or(sender, |(name, _)| name);
    let trimmed = head.trim().trim_matches('"');
    if !trimmed.is_empty() {
        return trimmed.to_owned();
    }
    sender.replace(['<', '>'], "").split('@').next().unwrap_or("").to_owned()
}

/// Title of a list filter (filterTitle).
pub fn filter_title(filter: &str) -> &'static str {
    match filter {
        "all" => "All",
        "unread" => "Unread",
        "flagged" => "Important",
        "today" => "Today",
        _ => "All",
    }
}

/// Title of a rule field (fieldTitle).
pub fn field_title(field: &str) -> &'static str {
    match field {
        "sender" => "Sender",
        "domain" => "Sender's domain",
        "subject" => "Subject",
        "text" => "Text",
        "account" => "Account",
        _ => "Sender",
    }
}

/// Words of a rule action (ruleLine).
pub fn rule_line(action: &Value) -> String {
    match action["kind"].as_str().unwrap_or("") {
        "junk" => "Move to Junk".to_owned(),
        "important" => "Mark important".to_owned(),
        "markRead" => "Mark as read".to_owned(),
        "move" => format!("Move to {}", action["mailbox"].as_str().unwrap_or("")),
        _ => String::new(),
    }
}

/// Past-tense words of an action in the journal (journalLine).
pub fn journal_line(action: &Value) -> String {
    match action["kind"].as_str().unwrap_or("") {
        "junk" => "Moved to Junk".to_owned(),
        "important" => "Marked important".to_owned(),
        "markRead" => "Marked as read".to_owned(),
        "move" => format!("Moved to {}", action["mailbox"].as_str().unwrap_or("")),
        _ => String::new(),
    }
}

/// Words of a triage reason (reasonLine).
pub fn reason_line(reason: &Value) -> String {
    let value = reason["value"].as_str().unwrap_or("");
    match reason["kind"].as_str().unwrap_or("") {
        "vip" => format!("VIP {value}"),
        "banned" => format!("banned word \"{value}\""),
        "rule" => format!("rule \"{value}\""),
        "manual" => "by hand".to_owned(),
        _ => String::new(),
    }
}

/// Split an answer into prose and whether it held a mail plan (splitPlan).
pub fn split_plan(text: &str) -> Value {
    let Some(start) = text.find(PLAN_FENCE) else {
        return json!({"prose": text, "plan": false});
    };
    let after = &text[start + PLAN_FENCE.len()..];
    let end = after.find("```");
    let mut prose = text[..start].to_owned();
    if let Some(end) = end {
        prose.push_str(&after[end + 3..]);
    }
    json!({"prose": prose.trim(), "plan": end.is_some()})
}

/// Name of a new rule (ruleName).
pub fn rule_name(field: &str, value: &str, action: &Value) -> String {
    format!("{} contains \"{}\" → {}", field_title(field), value, rule_line(action))
}

/// Whether a message matches a rule condition.
pub fn matches_condition(message: &Value, condition: &Value) -> bool {
    let field = condition["field"].as_str().unwrap_or("");
    let needle = condition["value"].as_str().unwrap_or("").to_ascii_lowercase();
    let value = match field {
        "sender" => sender_name(message).to_ascii_lowercase(),
        "domain" => message["address"].as_str().unwrap_or("").split('@').nth(1).unwrap_or("").to_ascii_lowercase(),
        "subject" => message["subject"].as_str().unwrap_or("").to_ascii_lowercase(),
        "account" => message["account"].as_str().unwrap_or("").to_ascii_lowercase(),
        "text" => message["body"].as_str().unwrap_or("").to_ascii_lowercase(),
        _ => String::new(),
    };
    !needle.is_empty() && value.contains(&needle)
}

/// First enabled rule matching a message (matchingRule).
pub fn matching_rule(message: &Value, rules: &[Value]) -> Value {
    rules
        .iter()
        .find(|rule| {
            rule["enabled"] == true
                && rule["conditions"]
                    .as_array()
                    .is_some_and(|conditions| conditions.iter().all(|condition| matches_condition(message, condition)))
        })
        .cloned()
        .unwrap_or(Value::Null)
}

/// One line describing a message (mailLine).
pub fn mail_line(message: &Value) -> String {
    format!(
        "{} · {} · {}",
        sender_name(message),
        message["subject"].as_str().unwrap_or(""),
        if message["unread"] == true { "unread" } else { "read" }
    )
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(messages: usize, unread: usize, rules: usize, enabled: bool) -> Vec<(&'static str, String)> {
    vec![
        ("messages", count_label(messages)),
        ("unread", count_label(unread)),
        ("rules", count_label(rules)),
        ("enabled", if enabled { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ids_and_senders() {
        assert_eq!(MAIL_ID, "integration_mail");
        assert_eq!(MAIL_BLUE, "#2F8CFF");
        assert_eq!(MAIL_FILTERS.len(), 4);
        assert_eq!(MAIL_FIELDS.len(), 5);
        assert_eq!(PLAN_FENCE, "```coucou-mail");
        let message = json!({"storeId":"inbox","entryId":"1","senderName":"","address":"ana@x.com"});
        assert_eq!(message_id(&message), "inbox/1");
        assert_eq!(sender_name(&message), "ana");
        assert_eq!(name_of("\"Ada\" <ada@x.com>"), "Ada");
        assert_eq!(name_of("<ada@x.com>"), "ada");
    }

    #[test]
    fn titles_and_actions() {
        assert_eq!(filter_title("unread"), "Unread");
        assert_eq!(field_title("domain"), "Sender's domain");
        assert_eq!(rule_line(&json!({"kind":"junk"})), "Move to Junk");
        assert_eq!(rule_line(&json!({"kind":"move","mailbox":"Archive"})), "Move to Archive");
        assert_eq!(journal_line(&json!({"kind":"markRead"})), "Marked as read");
        assert_eq!(reason_line(&json!({"kind":"vip","value":" Ana"})), "VIP  Ana");
        assert_eq!(reason_line(&json!({"kind":"manual"})), "by hand");
        assert_eq!(rule_name("sender", "news", &json!({"kind":"junk"})), "Sender contains \"news\" → Move to Junk");
    }

    #[test]
    fn plans_and_rules() {
        let split = split_plan("hello\n```coucou-mail\nplan\n```\nbye");
        assert_eq!(split["plan"], true);
        assert_eq!(split["prose"], "hello\nbye");
        assert_eq!(split_plan("plain")["plan"], false);
        let message =
            json!({"senderName":"News","address":"news@x.com","subject":"Hello","account":"a1","unread":true});
        let rules = vec![
            json!({"id":"1","enabled":true,"conditions":[{"field":"domain","value":"x.com"}]}),
            json!({"id":"2","enabled":false,"conditions":[{"field":"subject","value":"Hello"}]}),
        ];
        assert_eq!(matching_rule(&message, &rules)["id"], "1");
        assert!(matches_condition(&message, &json!({"field":"subject","value":"hello"})));
    }

    #[test]
    fn presentation() {
        let message = json!({"senderName":"Ana","subject":"Hi","unread":true});
        assert_eq!(mail_line(&message), "Ana · Hi · unread");
        assert_eq!(ACTION_KINDS.len(), 4);
        assert_eq!(REASON_KINDS.len(), 4);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 3, true).len(), 4);
    }
}
