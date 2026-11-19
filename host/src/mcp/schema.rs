//! Normalize JSON Schema field types in a server's `tools/list` reply before an agent sees them.
//!
//! Ported from OpenBot 0.74.0: a tool may describe a field as `true` or `false`, which JSON Schema
//! permits as `"type": "boolean"`. Some servers spell that `"bool"` or only give an enum of the two
//! values; an agent that received those failed the server test with "Invalid input" and the server
//! was not added. Normalizing here, at the proxy boundary, keeps the declared schema otherwise
//! untouched.

use serde_json::Value;

/// Rewrites `tools/list` replies; every other reply is returned as it came from the server.
pub(super) fn normalize_tools_reply(reply: &Value) -> Value {
    if !reply["result"]["tools"].is_array() {
        return reply.clone();
    }
    let mut normalized = reply.clone();
    if let Some(tools) = normalized["result"]["tools"].as_array_mut() {
        for tool in tools {
            if let Some(schema) = tool.get_mut("inputSchema") {
                normalize(schema);
            }
        }
    }
    normalized
}

/// Walks a schema recursively: `properties`, `items`, `$defs`, `anyOf` and the rest all go through
/// the same rule, so a boolean buried in a nested object is fixed too.
fn normalize(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            let is_boolean = map.get("type").is_some_and(type_is_boolean)
                || map
                    .get("enum")
                    .and_then(Value::as_array)
                    .is_some_and(|values| values.len() == 2 && values.iter().all(|v| v.as_bool().is_some()));
            if is_boolean && !map.contains_key("type") {
                map.insert("type".into(), Value::String("boolean".into()));
            }
            if let Some(kind) = map.get_mut("type") {
                normalize_type(kind);
            }
            for value in map.values_mut() {
                normalize(value);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize(item);
            }
        }
        _ => {}
    }
}

fn type_is_boolean(kind: &Value) -> bool {
    match kind {
        Value::String(s) => s.eq_ignore_ascii_case("bool") || s.eq_ignore_ascii_case("boolean"),
        Value::Array(types) => types.iter().any(type_is_boolean),
        _ => false,
    }
}

fn normalize_type(kind: &mut Value) {
    match kind {
        Value::String(s) if s.eq_ignore_ascii_case("bool") => *s = "boolean".into(),
        Value::String(s) if s.eq_ignore_ascii_case("boolean") && s != "boolean" => *s = "boolean".into(),
        Value::Array(types) => {
            for kind in types {
                normalize_type(kind);
            }
            types.dedup();
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reply_with(schema: Value) -> Value {
        json!({"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"tool","inputSchema":schema}]}})
    }

    #[test]
    fn boolean_fields_are_normalized() {
        let reply = reply_with(json!({"type":"object","properties":{
            "flag":{"type":"bool"},
            "other":{"type":"BOOLEAN"},
            "derived":{"enum":[true,false]},
            "nested":{"type":"object","properties":{"deep":{"type":["bool","null"]}}}
        }}));
        let out = normalize_tools_reply(&reply);
        let props = &out["result"]["tools"][0]["inputSchema"]["properties"];
        assert_eq!(props["flag"]["type"], "boolean");
        assert_eq!(props["other"]["type"], "boolean");
        assert_eq!(props["derived"]["type"], "boolean");
        assert_eq!(props["nested"]["properties"]["deep"]["type"], json!(["boolean", "null"]));
    }

    #[test]
    fn non_boolean_schemas_and_other_replies_are_untouched() {
        let schema = json!({"type":"object","properties":{"text":{"type":"string"},"count":{"type":"integer"}}});
        assert_eq!(normalize_tools_reply(&reply_with(schema.clone()))["result"]["tools"][0]["inputSchema"], schema);
        let call = json!({"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"a bool was mentioned"}]}});
        assert_eq!(normalize_tools_reply(&call), call);
    }
}
