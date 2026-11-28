# OpenBot 0.80.0 in Codync

Source: tag `v0.80.0` (97 bullets) plus the 13 fragments on `origin/main@383a35d2f`, audited against
the 226 commits already on this branch. Rule: copy behavior with a short source comment, reuse
Codync's code/APIs/components, pure tested decisions wired to real call sites. Excluded with reasons:
credits/prices, Chrome extension/X, joined servers, hosted/account service, web client, OS packaging,
performance-only internals.

## State

The audit found almost all v0.80.0 content outside Codync's scope: credit prices and packs, the NEAR
wallet and payments, VAT, Chrome/Web Store, joined-server gates, Apple Watch, the openbot.run site,
agent-dock/niri/Hyprland/Kaspersky paths, crowd-collaboration workflow cursors, and perf-only work
(mailbox/SQLite cache, image memory, typing lag, layout measurement). The 13 fragments are likewise
almost entirely in those buckets.

Already integrated, do not redo: multi-question answer mapping (`f7b9093fc`), quoted file names in
WhatsApp/Telegram replies (`31debc7ca`), WhatsApp stale-link recovery (`recovery.rs`), font pickers,
wide-table H-scroll, skill Save/Update retry, audio cards, paused routine rows, multi-select Library,
file-only page previews, spoken-text scrub and live-voice dedupe (`8e2da9025`, `c2eac8b1c`), workflow
canvas/nodes/history/versions, roster tabs, link guidance and selection actions.

## Portable candidates from the audit

These were identified as real or uncertain gaps; the strongest are handled first.

1. `[redacted]` hiding a normal word when a short secret is a substring — `host/src/diagnostics.rs` redacts whole whitespace words; redact only the exact secret span and refresh on Vault deletion.
2. An `<execute>` block written as text and not run must show "Failed" with the fixed reason — `host/src/agent/backends/**` + turn error display.
3. A damaged provider session file must start a fresh provider session with the chat history — `host/src/agent/acp/**`.
4. An MCP server that refuses a request for lack of a key should ask for the key with a card — `host/src/mcp/**` + desktop MCP settings.
5. A skill suggestion after a chat uses the language written in the chat — `host/src/chat/skill_proposals/**`.
6. A Marketplace app keeps its full address when settings are imported — `host/src/import/**` + `market/**`.
7. A very large ACP tool update must not wedge the provider afterwards; raise the byte cap and recover — `host/src/agent/acp/**`.
8. A slow app start waits up to 30 s in a workflow step — `host/src/routines/mcp.rs` (chats unchanged).
9. Font lists show cached fonts immediately when reopened — `apps/desktop/src/renderer/lib/prefs.ts` + font picker.
10. Collapsed sidebar popover shows the full name and truncates a long Title badge — desktop roster + phones.
11. "Too many images" refusal gets a bounded, actionable hint instead of "Try again" — `turn_rules.rs` + chat error cards.
12. Transparent-window Threads page fills behind the composer only while threads scroll under it — desktop thread CSS.
13. Manual chat sort keeps a dragged chat inside its own folder — desktop roster.
14. A timed-out provider check stops its whole child process tree — `host/src/agent/process/**`.
15. An ACP download with more than one program makes every binary executable — `host/src/agent/acp/**`.

## Close-out (commits 237-239)

All 15 candidates are resolved. #1 public-header redaction, #11 image-limit hint, #12 composer
fill, #13 manual folder drop, #14 process tree stop and #15 all ACP binaries executable are
implemented; #2 executed-as-text answer, #3 damaged ACP session and #4 MCP key card have pure
decisions wired to the turn/session/denial paths; #5 skill suggestion language and #9 cached font
list are wired in host and desktop. #6 needed no change: `market::custom` already keeps the full
MCP address on import; #7 is covered by the tool-update size decision; #8 was already present;
#10 popover name is wired in the desktop roster.

## Not portable (with reason)

Credits/prices/packs/payments/NEAR wallet/VAT; Chrome Web Store and extension promos; joined servers
and their version gates; hosted account service and email rate limits; web client and openbot.run;
Apple Watch; OS packaging (niri, Hyprland, Kaspersky, macOS/Linux executable bits); agent dock;
crowd-collaboration cursors; and performance-only internals.
