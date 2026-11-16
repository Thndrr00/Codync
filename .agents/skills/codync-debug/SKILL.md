---
name: codync-debug
description: Diagnose a Codync failure (host, agent, MCP, connector, pairing, voice, channel) from logs, diagnostics and the protocol, without guessing. Use when something fails and the cause is not obvious from the UI.
---

# Debugging Codync

Read the reason before changing code. The host records most failures; the clients only show a summary.

## Where the evidence is

- **Host log**: the terminal running `codync-host serve` (or the packaged service log). Provider and
  agent failures are also written to `logs/providers/providers.log`; voice failures to `logs/voice.log`.
- **Diagnostics**: the desktop's Settings → Diagnostics and the host's `diagnostics` method (version,
  data directory, database, ports, agents, health). `host/src/doctor.rs` has the same checks.
- **Health**: `hostHealth` (RAM/CPU/disk, refusing), thresholds in `host/src/health.rs`.
- **Turns**: the chat's trace (`views/thread/TraceView.tsx`; `t` in the TUI) shows each tool call and
  its result; analytics keeps `turn_finished` with the error code and reason.
- **Audit**: Settings → Audit log records security decisions.

## Order of checks

1. **Version/protocol**: is the app the same generation as the host? A stale host or app is the first
   suspect after switching branches; stop the old process and rebuild.
2. **Reachability**: loopback API up (`/api/hello`), token present, device paired and its scopes
   (Control/Screen) still valid; `devices` and `accessRequests` show the state.
3. **Agent**: did the agent start, sign in and advertise models? `agentAuth`, `agentModels`, and the
   provider log. A deleted downloaded agent shows as not installed; reinstall it.
4. **MCP/connector**: the connector's target and runtime; a vault-locked host can't read credentials
   (unlock it). `connectorCall` refusal text names the reason.
5. **Feature-specific**: voice (local server URL/key, provider key, logs/voice.log), channels (token,
   allow list, group mention), routines (trigger match, local script 429, flow skip), WhatsApp (bridge
   connection, allow list, pairing code).

## Rules

- Reproduce with the smallest flow and keep the time window of the log.
- Never paste secrets into a ticket; the host redacts them in logs, the UI truncates them.
- If the failure is an HTTP status, read the host's error body first: most refusals name the fix.
- A hang in a turn is usually a waiting approval card or a dead agent process; check both before
  restarting the host.
