# OpenBot unreleased fragments — December 2026 (post v0.78.0)

Source: the fragments added to OpenBot `origin/main` after `v0.78.0` (at `ac2738d04`), read from
`changelog.d/`. Same rule as earlier plans: copy behavior and user-facing text with a short source
comment, reuse Codync's own components and host APIs, bridge where an OpenBot subsystem does not
exist. Four auditors also covered the v0.78.0 release bullets; see `openbot-0.78.0.md`.

## State

All eight portable gaps are integrated in three disjoint batches and pushed. Each has a pure tested
decision wired to a real call site; six new labels are in the eleven catalogs. Nothing here has been
compiled or run end to end on this machine. Static checks pass: `rustfmt`, `tsc`, the focused pure
tests and catalog parsing. The full desktop suite's four failures remain the pre-existing
locale-sensitive assertions on this Spanish-locale machine.

## Portable — in progress

| Fragment | Codync bridge |
| --- | --- |
| `triage-15811` — OpenCode silently works without Codync's tools | Detect tools missing on session start; say so and re-declare next message |
| `triage-15865` — one corrupt stored item must not block startup | Skip and log only the bad row in the entry loaders |
| `triage-15916` — deleting an agent purges its own channel chats | Delete that bot's entries in channel lanes, keep other members' messages |
| `triage-15917` — ChatGPT agent always uses the chosen reasoning effort | Write the chosen effort into Codync's `CODEX_CONFIG` for the session |
| `triage-15934` — Workspace-only blocked write says "Try Full access" | Fixed hint in the denial card/text |
| `triage-15940` — plan mode ends with Approve / Stop planning and free text | Extend the existing `plan_card` and answer commit |
| `triage-15938` — a website `alert()` closes itself and the agent gets its text | Handle `Page.javascriptDialogOpening` in the CDP loop |
| `triage-15839` — invalid reply moves a backup-model agent on | Classify invalid replies before work starts, jump to next automatic backup |

## Already covered

- `triage-15851` — slow custom agent's models: Codync probes with a one-minute timeout and keys the
  in-flight future per backend, so other providers never wait and the models still appear.

## Not ported — no Codync equivalent

- **Credits and the account service** (`t3-redeem-credit-codes-confetti`, `triage-15930`): no credit
  wallet, redemption codes or hosted account service; the credits page and confetti have no surface.
- **Joined servers** (`triage-15812`, `15817`, `15846`, `15853`): server IPv4 routing, Retry keeping
  channels/tasks, per-server sign-ins, idle Gemini process notices: Codync's host owns the logic and
  has no joined-server protocol; its relay and certificate stack are its own.
- **Hosted servers** (`triage-15812` IPv6 fix applied by the hosting team): no hosted product.
- **Local-script wake ports** (`triage-15811` second half): Codync's headless runner takes an explicit
  `--port` and has no stale port-file discovery.
