# OpenBot unreleased fragments — November 2026

Source: the nine files under `changelog.d/` on OpenBot's `origin/main` at `20d1cd63d`, fetched after
`v0.77.0`. Same rule as the earlier plans: copy behavior and user-facing text with a short source
comment, reuse Codync's own components, and bridge to Codync concepts where an OpenBot subsystem does
not exist. "Joined server" and web/extension items need a client Codync doesn't have.

## State

All seven portable fragments are integrated in four disjoint batches on this branch and pushed:
MCP test-connection tool restore, overlapping routine/agent questions, provider capability messages
and Grok's empty sessions, and duplicate-agent copying plus the sign-in handoff. Five new labels are
in the eleven catalogs.

Nothing here has been compiled or run end to end on this machine. Static checks pass: `rustfmt`,
`tsc`, the pure tests and catalog parsing. The desktop suite's four failures are the same
locale-sensitive assertions seen on this Spanish-locale machine.

## Portable fragments

| Fragment | Codync bridge |
| --- | --- |
| `triage-15335` — a passing Test connection gives a chat its MCP tools back from the next message, signed in or not | The existing MCP verify path and per-turn tool declarations |
| `triage-15338` — a routine question no longer closes an earlier agent question in the same chat | Independent card lifecycles in the external card state and the desktop card list |
| `triage-15353` — Grok model checks no longer leave an empty session | The periodic model-list wrapper without a session |
| `triage-15358` — an unsupported processor says the provider can't run here, not "Download failed" | Provider capability classification on the Providers row |
| `triage-15344` — Claude Connect/Add account opens the sign-in page on every OS | The same connector sign-in page path the other providers use |
| `triage-15376` — duplicate keeps linked folders and retries when Windows holds the copy | The real `duplicate_bot` folder copy with bounded retry |
| `triage-15384` — a slow sign-in navigation doesn't hand the browser to the user | The browser submit/navigation path with a bounded grace period |

## Not ported — no Codync equivalent

- **`t3-070aa823` — credit purchases alert a Discord webhook.** Codync has no credit wallet or
  operator alert channel.
- **`t3-94d7beb2` — the Chrome extension.** The side panel with pinned agents, chats, search and
  threads, X replies with a seven-day retry queue, extension dictation, channels/folders, agent
  settings and shared Chrome tabs all live in a browser extension Codync does not build. The
  underlying capabilities exist in Codync's own clients (phone-style roster, dictation, channels,
  bot editor, and agents using their own browser behind permission cards), but there is no side
  panel to put them in and no X connector to receive those replies.
