---
name: codync-integrate
description: Add or change a Codync feature the Codync way: host method or tool first, then every client, shared strings, tests and docs. Use when implementing a feature that touches the host API or more than one client.
---

# Integrating a feature into Codync

The host owns the logic; clients render host state and send intents. A client never grows its own copy
of a feature the host could answer.

## Order

1. **Host first.** Add the state, validation and method (or MCP tool) in `host/src`. Put it behind the
   host API (`host/src/api`), with the caller permissions in `host/src/api/devices.rs` (local-only,
   device control, screen). Validate input in the host even when the client already checks it.
2. **Wire the protocol.** Add the method name to the API dispatcher and to any vault-unlock or
   analytics branch it needs. Return JSON that every client can decode; bump nothing if the change is
   additive, but a removed or renamed field raises the phone-host `minApp`/`minHost` in the same
   release (see `docs/reference/compatibility.md`).
3. **Every client, same change.** Desktop (`apps/desktop/src/renderer`), iPhone
   (`apps/ios` + `apps/ios/Kit/Sources/CodyncUI`), Android (`apps/android`), and the terminal
   (`host/src/tui`). Keep labels, empty/loading/error states and confirmation behaviour aligned; adapt
   layout and input to each platform.
4. **Shared strings.** Add the English key to every catalog at
   `apps/ios/Kit/Sources/CodyncKit/Resources/Languages/*.json` (including `ar.json` and `he.json`),
   CRLF, no BOM, same keys as `es.json`, with real translations. Use a Node script for the edit; the
   key parity test lives in `apps/desktop/src/renderer/lib/catalogs.test.ts`.
5. **Tests.** Host unit tests for pure logic and the refusal paths; client tests where the repo tests
   pure helpers. Name tests after observable behaviour.
6. **Docs.** Update the feature doc in `docs/features/` and the page of the user manual in
   `web/content/docs/` in the same change.

## Controls and conventions

- Use Codync's own controls, not platform defaults: iOS `Controls.swift`/`Chrome.swift`, desktop
  `components/` (`Sheet`, `Dialog`, `AnchoredMenu`, `ModalHeader`, `Controls.tsx`, `Icon`), Android
  `design/`, TUI sheets and keys. Native navigation/toolbars only where `docs/design/ui-conventions.md`
  allows it.
- Four-space Swift/Rust, two-space TypeScript. Keep files under ~500 lines; split by responsibility.
- No I/O in views. No swallowed errors. Enums over flags.
- The iPhone and Android apps are kept in lockstep: a change to one lands in the other in the same
  change, checked on both.

## Before finishing

Run the platform checks for what you touched, then the after-change skill (review → fix → e2e). Do not
reformat or revert other agents' hunks in a shared checkout.
