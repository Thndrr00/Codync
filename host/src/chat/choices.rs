//! Choice cards: a decision a person answers with one click (agentes.chat ADR 0124).
//!
//! A card holds a question and a bounded set of labeled options. It is stored with the same kv
//! helper the other chat features use, and answering it once records the choice — a later answer
//! cannot change it.

use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::{Value, json};

const MAX_OPTIONS: usize = 8;
const MAX_LABEL_CHARS: usize = 120;

fn key(id: &str) -> String {
    format!("choice-card:{id}")
}

/// A card's options: at least two, each with an id and a short label.
pub fn normalize(options: &Value) -> Result<Vec<Value>> {
    let list = options.as_array().ok_or_else(|| anyhow::anyhow!("options must be an array"))?;
    if list.len() < 2 || list.len() > MAX_OPTIONS {
        bail!("a choice card needs between 2 and {MAX_OPTIONS} options");
    }
    let mut out = Vec::new();
    for option in list {
        let id = option["id"].as_str().unwrap_or_default().trim();
        let label = option["label"].as_str().unwrap_or_default().trim();
        if id.is_empty() || label.is_empty() {
            bail!("every option needs an id and a label");
        }
        if label.chars().count() > MAX_LABEL_CHARS {
            bail!("an option label must be at most {MAX_LABEL_CHARS} characters");
        }
        out.push(json!({"id": id, "label": label}));
    }
    Ok(out)
}

/// Creates a card from a question and its options.
pub fn create(store: &Store, id: &str, question: &str, options: &Value) -> Result<Value> {
    let question = question.trim();
    if question.is_empty() {
        bail!("a choice card needs a question");
    }
    let options = normalize(options)?;
    let card = json!({"question": question, "options": options, "answer": Value::Null});
    store.kv_set(&key(id), &serde_json::to_string(&card)?)?;
    Ok(card)
}

/// Answers a card once. A second answer is refused and leaves the first intact.
pub fn answer(store: &Store, id: &str, option: &str) -> Result<Value> {
    let text = store.kv_read(&key(id)).ok().flatten().ok_or_else(|| anyhow::anyhow!("unknown choice card"))?;
    let mut card: Value = serde_json::from_str(&text)?;
    if !card["answer"].is_null() {
        bail!("this choice card was already answered");
    }
    if !card["options"].as_array().is_some_and(|o| o.iter().any(|v| v["id"] == option)) {
        bail!("that option is not on this card");
    }
    card["answer"] = json!(option);
    store.kv_set(&key(id), &serde_json::to_string(&card)?)?;
    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Value {
        json!([{"id":"yes","label":"Yes"},{"id":"no","label":"No"}])
    }

    #[test]
    fn options_need_ids_labels_and_a_reasonable_count() {
        assert_eq!(normalize(&options()).unwrap().len(), 2);
        assert!(normalize(&json!([{"id":"only","label":"One"}])).is_err());
        assert!(normalize(&json!([{"id":"a"},{"id":"b","label":"B"}])).is_err());
    }
}
