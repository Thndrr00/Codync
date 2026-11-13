# Credential inspection

`governance.rs` is a port of `content-governance.ts` in `CopilotKit/OpenBot` (the credential
categories, field spellings, provider formats, path rules and size limits), MIT-licensed,
Copyright (c) 2025 CopilotKit.

What Codync changed: the module is Rust over a `serde_json::Value` rather than TypeScript over an
in-process object, so the `WeakSet` cycle guard is dropped (parsed JSON cannot be cyclic) and the
arguments come from ACP's `tool_call`/`tool_call_update` `rawInput` instead of a gateway route. The
finding rides on the tool entry (`caution`) and is drawn on its row, before and after the call,
instead of refusing it or writing an audit row.
