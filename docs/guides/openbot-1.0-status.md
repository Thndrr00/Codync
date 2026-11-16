# OpenBot (CopilotKit) 1.0 → Codync: status

Reference: `github.com/CopilotKit/OpenBot` (MIT app), latest tag v1.0.3 (2026-11-14), plus the
nightly `nightly-labs/openbot` 0.65.0 for the earlier F cards. This page tracks what came over and
what is deliberately left out; it is not a roadmap promise.

## In Codync

| OpenBot capability | Codync home |
| --- | --- |
| AG-UI agents (server and provider) | `host/src/agui`, backend `agui` |
| A2A between computers | `host/src/a2a`, network letters |
| Channels: Slack, Teams, Signal, Matrix, Discord, Telegram, WhatsApp | `host/src/{slack,teams,signal,matrix,discord,telegram,whatsapp}` |
| Native channel approvals (Block Kit, Adaptive Cards, components, keyboards, numbered) | `host/src/channels/render.rs` |
| Policy: named CEL rules, deny before allow, fail closed, presets, what-if | `host/src/policy` |
| People, roles, device binding, member-scoped MCP | `host/src/people.rs` |
| Invites, sessions and member sign-in | in progress |
| Routine safeguards: 15-minute floor, 20 enabled, pause after 10 failures | `host/src/routines/guardrails.rs` |
| Component gallery and playground | `host/src/components` |
| Declarative blocks (A2UI-style) | `host/src/declarative` |
| View state and named actions | `host/src/views` |
| Dictation threads, local voice server, Supertonic, xAI, Groq | `host/src/voice` |
| Commit co-author trailer | `host/src/chat/commit_coauthor.rs` |
| Docker/Compose deployment | `Dockerfile`, `docker-compose.yml` |
| Offline mocks (provider, MCP, A2A, AG-UI) | `host/src/testing` |

Earlier F cards (0.58–0.65) are recorded in `docs/guides/openbot-gap-census.md`.

## Added after the first pass

- Member accounts with invite links or company email, a Control-scoped
  session tied to a person, and revocation.
- A browser web client in `web/app/app` with a loopback bridge.
- Per-person/bot Docker containers with gVisor when available and a local
  fallback, plus Automatic Learning with approval before saving a fact.

## Out of scope, with reasons

- **Hosted servers and paid plans**: OpenBot sells hosting; Codync has its own cloud and computers.
  The local mini-app limit (`F11.16`) is a guardrail, not a plan.
- **CopilotKit Intelligence**: separately licensed; Codync's memory, beliefs and routines are its own.
- **Full Kubernetes/Helm workspace**: Codync's host is one binary; the Docker image is the supported
  self-hosting path.
- **Native model runtime**: a local OpenAI-compatible voice server covers dictation/TTS.

## Validation

No builds, tests, typechecks or end-to-end runs have been executed on this branch by instruction
("no compiles"). Before merge: `cargo fmt/clippy/test`, desktop `npm run typecheck && npm test`, the
Gradle checks, the iPhone package tests, the app walkthroughs, and the manual `cloud/` deploys.
