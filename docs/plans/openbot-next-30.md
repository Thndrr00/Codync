# Next 30 portable items into Codync

Source: 2,350 changelog bullets from OpenBot `v0.66.0`–`v0.77.0` plus the unreleased fragments on
`origin/main`, audited against the 182 commits already integrated on this branch. Rule: copy behavior
and user-facing text with a short source comment, reuse Codync's own components and host APIs, add a
pure tested decision, bridge where Codync's subsystem has a different shape. No hosted/billing/credits,
joined servers, web client, Chrome extension, X, OS packaging or SolidJS.

## State

Twenty-eight items were integrated in seven batches and pushed. Each has a pure tested decision wired
to a real call site. The few rows still open are listed with a reason at the end. Static checks pass:
`rustfmt`, `tsc`, the focused pure tests and the eleven catalogs. The full desktop suite's four
failures are the pre-existing locale-sensitive assertions on this Spanish-locale machine. Nothing was
compiled or run end to end here (no memory for a build), as requested.

## Integrated — host and UI

| # | Item | Where |
| --- | --- | --- |
| 1 | Routine run prompt names the local date/time in the routine's timezone | `routines/schedule.rs`, `routines/runner.rs` |
| 2 | A reaction on a bot reply is also your answer to its open question | `hub/reactions.rs`, external cards |
| 3 | A routine question no longer hides an earlier agent question | `external/question.rs`, desktop card list (commit 179) |
| 4 | A failed "Asked" row clears when the teammate is asked again | `views/thread/waiting-replies.ts` |
| 5 | Paused routines show once per day with a paused mark | `routines/schedule_calendar.rs`, schedule UI |
| 8 | Skill-suggestion Save/Update fails fast and stays retryable when busy | `chat/skill_proposals.rs` |
| 11 | A new agent opens with a welcome question and three first tasks | `agent/new_bot.rs`, `api/bots.rs` |
| 13 | A recently errored backup shows "Not available now · skipped" | `agent/fallback/reason.rs`, desktop bot UI |
| 15 | Audio attachments get an in-chat player on desktop and iOS | `views/thread/AudioCard.tsx`, iOS `Attachments.swift` |
| 30 | Library multi-select with a Download/Delete bar | `LibraryView.tsx`, `api/library_batch.rs` |
| 31 | Interface and code font pickers in Settings | `prefs.ts`, `theme.css`, main font IPC |
| 32 | Markdown task-list checkboxes and an Image unavailable fallback | chat markdown renderers |
| 33 | PDF and HTML file links open in the in-app side panel | `chat-file-link.ts`, `PagePreview.tsx` |
| 34 | Wide tables and long code scroll sideways with edge affordances | `views/thread/HScroll.tsx` |
| 36 | Back walks Settings and Marketplace before closing a sheet | `chat-navigation.ts`, both views |
| 37 | A Files panel top row reveals the bot's own workspace | `BotFilesSheet.tsx`, narrow reveal IPC |

## Integrated — host behavior (no new UI surface)

| # | Item | Where |
| --- | --- | --- |
| 16 | An agent gets the file of the WhatsApp message you reply to | `whatsapp-bridge/bridge.mjs`, `whatsapp/incoming.rs` |
| 17 | An agent can attach a browser screenshot to a reply | `screen/browser_tools`, `chat/files.rs` |
| 18 | A page with no text gets a picture instead of an empty snapshot | `screen/browser_tools/cdp.rs` |
| 19 | Agent drag-and-drop inside a browser page | `browser_tools/cdp.rs` |
| 20 | Reload bypasses a broken cached resource | `browser_tools/cdp.rs`, `browser_panel.rs` |
| 21 | Snapshot has a drawn pass for content hidden from screen readers | `browser_tools/page.js` |
| 22 | Fields named `hidden` or `inert` are no longer skipped | `page.js`, `protected-fill.js` |
| 23 | Generated media is named from the first words of its prompt | `media/mod.rs`, `media/flow.rs`, `agent/bot/files.rs` |
| 24 | A skill's shebang/`.sh` scripts stay executable on install and revise | `market/skills.rs`, `skill_history.rs` |
| 29 | Re-reading history returns full replies while search stays bounded | `chat/memory/tools.rs` |
| 38 | UTF-8 BOM in `SKILL.md` no longer hides its front matter | `market/skills.rs` |
| 39 | A side question reads the thread it was asked in | `chat/team/side_question.rs` |

## Still open, with a reason

- **6 — roster row shows a picture's name**: a small host-preview change; not needed for the 20–30 target.
- **7 — 0-byte attachment refused**: clear value, needs a decision on wording in every composer.
- **9 — Read-aloud failure is visible**: Android only has the per-message Read aloud surface.
- **10 — a chat correction updates the routine that uses it**: needs a belief→routine mapping.
- **12 — mark a chat unread from the sidebar**: the host method exists; only the menus are left.
- **14 — opt-in "Learn from my browsing"**: a new off-by-default setting and site-memory capture.
- **25 — auth code detection reads label/placeholder**: the agent-driven half has no Codync surface.
- **26 — recreate shared-tables file after deletion**: no shared-tables tool exists in Codync.
- **27 — Stop ends a Gemini turn the provider no longer runs**: the existing stale-turn path covers most of it.
- **28 — read online videos**: no bundled downloader; spawning one would bypass egress pinning.
- **32 (part) — display formulas**: no math renderer in any client; would be its own change.
- **35 — new flow node in view / composer focus after new thread**: low-impact micro-fixes.

## Out of scope (unchanged)

Hosted/billing/credits, joined servers, web client, Chrome extension/X, Signal regions, OS packaging,
SolidJS — see `openbot-0.77.0.md` and `openbot-fragments-2026-11.md`.
