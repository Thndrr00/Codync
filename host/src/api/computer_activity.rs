//! Read-only activity APIs shared by local apps and authorized control devices.

use super::str_arg;
use crate::hub::Hub;
use crate::store::ComputerActivity;
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub(super) fn read(hub: &Hub, method: &str, body: &Value) -> Result<Value> {
    let bot = str_arg(body, "botId")?;
    if hub.store.bot(bot)?.is_none_or(|row| row.deleted) {
        bail!("unknown bot");
    }
    let items = hub.store.computer_activity(bot)?;
    if method == "exportComputerActivity" { Ok(json!({"csv": csv(&items)})) } else { Ok(json!({"items": items})) }
}

fn csv(items: &[ComputerActivity]) -> String {
    let mut text = String::from("id,botId,action,actorKind,actorId,result,createdAt\r\n");
    for item in items {
        let cells = [
            item.id.to_string(),
            cell(&item.bot_id),
            cell(&item.action),
            cell(&item.actor_kind),
            cell(&item.actor_id),
            item.result.as_str().to_owned(),
            item.created_at.to_string(),
        ];
        text.push_str(&cells.join(","));
        text.push_str("\r\n");
    }
    text
}

fn cell(value: &str) -> String {
    // Quoting alone doesn't prevent spreadsheet programs from evaluating formulas.
    let prefix = if value.starts_with(['=', '+', '-', '@', '\t', '\r', '\n']) { "'" } else { "" };
    format!("\"{prefix}{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ComputerActivityResult;

    #[test]
    fn activity_requires_control_scope_and_refuses_unpaired_devices() {
        use crate::api::devices::{Caller, permit};
        use crate::store::Scope;

        for method in ["listComputerActivity", "exportComputerActivity"] {
            assert!(permit(&Caller::Local, method).is_ok());
            assert!(permit(&Caller::Device { key: "a".into(), scopes: vec![Scope::Control] }, method).is_ok());
            assert!(permit(&Caller::Device { key: "a".into(), scopes: vec![Scope::Screen] }, method).is_err());
            assert!(permit(&Caller::Pairing { key: "a".into() }, method).is_err());
        }
    }

    #[test]
    fn export_quotes_fields_and_neutralizes_spreadsheet_formulas() {
        let row = ComputerActivity {
            id: 1,
            bot_id: "=cmd(\"x\"),\n".into(),
            action: "type_text".into(),
            actor_kind: "bot".into(),
            actor_id: "=cmd(\"x\"),\n".into(),
            result: ComputerActivityResult::Success,
            created_at: 123,
        };
        let text = csv(&[row]);
        assert!(text.contains("\"'=cmd(\"\"x\"\"),\n\""));
        assert!(text.ends_with(",success,123\r\n"));
    }
}
