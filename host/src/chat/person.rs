//! A profile per person, shared by every bot that talks to them (agentes.chat ADR 0052).
//!
//! Facts are what the person said about themselves — how they like answers, their role, their
//! projects. They are kept apart from a bot's own memory, bounded so they cost little in every
//! prompt, and inspectable and deletable so the person stays in control.

use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub const MAX_FACTS: usize = 30;
pub const MAX_FACT_CHARS: usize = 200;
pub const MAX_TOTAL_CHARS: usize = 1_200;

fn key(person: &str) -> String {
    format!("person-profile:{person}")
}

fn read(store: &Store, person: &str) -> Vec<String> {
    store
        .kv_read(&key(person))
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .unwrap_or_default()
}

fn write(store: &Store, person: &str, facts: &[String]) -> Result<()> {
    store.kv_set(&key(person), &serde_json::to_string(facts)?)
}

/// A fact as the person said it: trimmed, nonempty and bounded.
pub fn normalize_fact(text: &str) -> Result<String> {
    let text = text.trim();
    if text.is_empty() {
        bail!("a fact needs text");
    }
    if text.chars().count() > MAX_FACT_CHARS {
        bail!("a fact must be at most {MAX_FACT_CHARS} characters");
    }
    Ok(text.to_owned())
}

/// The facts known about a person, for settings and for the prompt.
pub fn list(store: &Store, person: &str) -> Vec<String> {
    read(store, person)
}

/// Adds a fact, keeping the list and its total character budget bounded. A duplicate is not added.
pub fn add(store: &Store, person: &str, text: &str) -> Result<Value> {
    let fact = normalize_fact(text)?;
    let mut facts = read(store, person);
    if facts.contains(&fact) {
        return Ok(json!({"facts": facts, "added": false}));
    }
    facts.push(fact);
    while facts.len() > MAX_FACTS {
        facts.remove(0);
    }
    while facts.iter().map(|f| f.chars().count()).sum::<usize>() > MAX_TOTAL_CHARS && facts.len() > 1 {
        facts.remove(0);
    }
    write(store, person, &facts)?;
    Ok(json!({"facts": facts, "added": true}))
}

/// Removes a fact. A deleted fact is simply gone: nothing re-learns it from the store.
pub fn remove(store: &Store, person: &str, text: &str) -> Result<Value> {
    let fact = normalize_fact(text)?;
    let mut facts = read(store, person);
    facts.retain(|f| f != &fact);
    write(store, person, &facts)?;
    Ok(json!({"facts": facts}))
}

/// The hidden block a run sees about the person behind its message.
pub fn render(facts: &[String]) -> String {
    if facts.is_empty() {
        return String::new();
    }
    let mut out = String::from("# About this person\n");
    for fact in facts {
        out.push_str("- ");
        out.push_str(fact);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fact_is_trimmed_bounded_and_nonempty() {
        assert_eq!(normalize_fact("  prefers short answers  ").unwrap(), "prefers short answers");
        assert!(normalize_fact("  ").is_err());
        assert!(normalize_fact(&"x".repeat(MAX_FACT_CHARS + 1)).is_err());
    }

    #[test]
    fn the_rendered_block_lists_each_fact_once() {
        let facts = vec!["prefers short answers".to_owned(), "owns the billing service".to_owned()];
        let block = render(&facts);
        assert!(block.starts_with("# About this person\n"));
        assert!(block.contains("- prefers short answers\n"));
        assert!(block.contains("- owns the billing service\n"));
        assert_eq!(render(&[]), "");
    }
}
