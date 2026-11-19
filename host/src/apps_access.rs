//! Public form links for bot-built apps, from bdots' `agents/app-access.ts`.
//!
//! A `submit` link can add items to the app's `submit` collections and read only the items
//! it added, by the 128-bit receipt it got back. It can also read the app's `read` collections.
//! That makes a public form possible without handing out the whole app or a Codync account.

use crate::store::AppLinkMode;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_COLLECTIONS: usize = 10;
pub const MAX_SUBMISSION_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Access {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub submit: Vec<String>,
    #[serde(default)]
    pub wake: bool,
}

fn clean(list: &[String]) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for name in list {
        let name = name.trim();
        if name.is_empty()
            || name.len() > 64
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!("a collection name must be 1-64 letters, digits, '_' or '-'");
        }
        let owned = name.to_owned();
        if !out.contains(&owned) {
            out.push(owned);
        }
    }
    if out.len() > MAX_COLLECTIONS {
        bail!("at most {MAX_COLLECTIONS} collections per public link");
    }
    Ok(out)
}

/// Validates and normalises a form's public collections. A submit collection cannot also be public to read.
pub fn validate(access: &Access) -> Result<Access> {
    let read = clean(&access.read)?;
    let submit = clean(&access.submit)?;
    if submit.is_empty() {
        bail!("a public form needs at least one submit collection");
    }
    if let Some(both) = submit.iter().find(|c| read.contains(c)) {
        bail!("'{both}' cannot be both public to read and a submit collection");
    }
    Ok(Access { read, submit, wake: access.wake })
}

pub fn new_receipt_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn is_receipt(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// What a form link may do with one collection, and the item id to store under.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Allow { receipt: Option<String> },
    Deny(u16, &'static str),
}

/// Authorises one data operation for a link. `op` is `list`, `get`, `put` or `delete`.
pub fn authorize(
    mode: AppLinkMode,
    access: &Access,
    op: &str,
    collection: &str,
    id: Option<&str>,
    bytes: usize,
) -> Decision {
    if mode == AppLinkMode::Edit {
        return Decision::Allow { receipt: None };
    }
    if mode != AppLinkMode::Submit {
        return if matches!(op, "list" | "get") {
            Decision::Allow { receipt: None }
        } else {
            Decision::Deny(403, "This link is view-only")
        };
    }
    match op {
        "list" if access.read.iter().any(|c| c == collection) => Decision::Allow { receipt: None },
        "get" if access.read.iter().any(|c| c == collection) => Decision::Allow { receipt: None },
        "get" if access.submit.iter().any(|c| c == collection) && id.is_some_and(is_receipt) => {
            Decision::Allow { receipt: None }
        }
        "put" if access.submit.iter().any(|c| c == collection) => {
            if id.is_some() {
                Decision::Deny(403, "A public link can only add new items")
            } else if bytes > MAX_SUBMISSION_BYTES {
                Decision::Deny(413, "That submission is too long")
            } else {
                Decision::Allow { receipt: Some(new_receipt_id()) }
            }
        }
        "list" | "get" => Decision::Deny(403, "This link cannot read that"),
        "put" => Decision::Deny(403, "A public link cannot change that"),
        _ => Decision::Deny(403, "A public link can only submit"),
    }
}

/// The key a form's public collections are stored under, beside the link itself.
pub fn access_key(bot_id: &str, slug: &str) -> String {
    format!("app.access.{bot_id}.{slug}")
}

pub fn read_access(hub: &crate::hub::Hub, bot_id: &str, slug: &str) -> Value {
    hub.store
        .kv_get(&access_key(bot_id, slug))
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::to_value(Access::default()).unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access() -> Access {
        validate(&Access { read: vec!["status".into()], submit: vec!["responses".into()], wake: false }).unwrap()
    }

    #[test]
    fn a_form_can_submit_but_not_change_or_read_other_items() {
        let a = access();
        assert_eq!(authorize(AppLinkMode::Submit, &a, "list", "status", None, 0), Decision::Allow { receipt: None });
        assert_eq!(
            authorize(AppLinkMode::Submit, &a, "list", "responses", None, 0),
            Decision::Deny(403, "This link cannot read that")
        );
        assert!(matches!(
            authorize(AppLinkMode::Submit, &a, "put", "responses", None, 10),
            Decision::Allow { receipt: Some(_) }
        ));
        assert_eq!(
            authorize(AppLinkMode::Submit, &a, "put", "responses", Some("mine"), 10),
            Decision::Deny(403, "A public link can only add new items")
        );
        assert_eq!(
            authorize(AppLinkMode::Submit, &a, "put", "status", None, 10),
            Decision::Deny(403, "A public link cannot change that")
        );
        assert!(matches!(
            authorize(AppLinkMode::Submit, &a, "delete", "responses", Some("x"), 0),
            Decision::Deny(403, _)
        ));
    }

    #[test]
    fn a_receipt_reads_its_own_submission() {
        let a = access();
        let id = new_receipt_id();
        assert!(is_receipt(&id));
        assert!(!is_receipt("b1"));
        assert_eq!(
            authorize(AppLinkMode::Submit, &a, "get", "responses", Some(&id), 0),
            Decision::Allow { receipt: None }
        );
    }

    #[test]
    fn read_and_submit_cannot_overlap() {
        assert!(validate(&Access { read: vec!["x".into()], submit: vec!["x".into()], wake: false }).is_err());
        assert!(validate(&Access { read: vec![], submit: vec![], wake: false }).is_err());
    }
}
