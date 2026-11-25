# Remaining OpenBot → Codync work — master inventory

Four read-only audits (197 changelog.d fragments on `origin/main@1ebeb279e`, plus full workflow
contract parity) against the 210 commits already on this branch. Rule: copy behavior with a short
source comment, reuse Codync's code/APIs/components, pure tested decisions wired to real call sites.
Excluded with reasons: joined servers, hosted/account service, web client, Chrome extension/X, OS
packaging, performance-only internals.

## State

In progress. Batches by disjoint domain below. Priorities: (1) workflow parity, which the user wants
exactly as OpenBot has it; (2) every other portable fragment. Nothing was compiled or run end to end.

## A — Workflow parity (23 capabilities)

### Node types and canvas
1. Standalone LLM/model step (provider, model, effort, prompt, system, answer).
2. Condition node with two routed yes/no outputs, not just a skip check.
3. Approval step that pauses a run for a human decision (decideWorkflowStep).
4. Output node with target+action (notify/channel/file/github/discord/telegram/slack; comment, label, assign, review, close, react, post, reply).
5. Annotation nodes (text, prompt, note, shape) with resizable size and font.
6. Asset/file input node feeding existing files into text/image/file ports.
7. Trigger node on the canvas itself (trigger/event/filters/connection/place/roles/text/answer; nextDueAt, webhookUrl).
8. Image step advanced inputs (custom width/height, first/last frame step links, background, provider options, mode with frames/references/videos/audios).

### Triggers and integrations
9. First-class GitHub trigger: 12 events, filters, 24 typed fields, delivery history with skip reasons, "run with last event".
10. Discord/Telegram/Slack trigger nodes by connection+channel+roles, failed reaction when all skip, "last event" test.
11. Workflow-level webhook trigger with own URL, secret rotation and receipts.

### Canvas UI and format
12. Freehand drawing (strokes: brush/color/width/points) persisted in the graph.
13. Per-node resize and font size.
14. Node locking (locked:true → read-only placeholder).
15. Typed ports (text/image/file) with connectivity validation and edge problem refusal.
16. User-visible version history and restore (auto/restore/run, up to 50, autosave 5 min).
17. Unknown node kinds as locked placeholder notes with needsNewerApp, not whole-workflow read-only.

### Runtime and answers
18. Generic retryStep of a failed attempt with a fresh requestId.
19. Stop a single step or one run by id.
20. Run history with per-step attempt, branch, waiting and source attachments.
21. Paginated, numbered run history (before/limit, nextBefore).
22. Run source recorded per run (manual/schedule/webhook/github/discord/telegram/slack).
23. Qualified field names `{{step.name}}` to disambiguate two suppliers.

## B — Portable fragments (87)

### Host / agent / turn
- A user message sent while a sign-in/password/code card is open waits; card stays answerable (15776).
- A routine wake of an open wait stays in main chat with separate-thread runs (15775).
- Teammates' later link/status not duplicated in your chat (16136); short "Starting now" note still delivers result (16168).
- Gemini background-command answer shown once (16149); optional-answer workflow step ends quietly (16368).
- Late message read by busy Claude requeues when the session closes (16361); Kimi self-continued reply reaches chat (16364).
- Compaction failure says so plainly with a 10-minute retry cap (15426).
- "Session already active" closes/reopens and retries (14354); provider stops before answering says so (15306).
- Reapply "After this turn" model change after running turns end (14455); keep chosen model across restart with slow model list (16212).
- Slow ChatGPT account check gets a bounded longer connect (16210); Vault reopens when storage becomes ready (16215).
- Channel Continue/Reassign/Cancel free the agent after unconfirmed start (16271).
- Bot mistral Workspace-only uses own tools without approval card (16398).
- Claude custom endpoint 402 names the service, not the plan (16172); Gemini duplicate tool results once (16149).

### Channels / messaging
- WhatsApp post rejected → line under the answer (14456), country-code re-add hint (13597).
- Telegram deletes "Working on it…" when the message isn't for the bot (15568); reply context already done.
- Channel preview shows newest real message not hidden steps (15542).
- Only first handoff in a request replies to your channel message (16003).
- Pipeline channel result from a side thread returns there (15775 area).

### MCP / providers / usage
- Saved-sign-in MCP servers marked "Sign-in needed"; Test connection asks (15267).
- MCP sign-in fields shown under Test connection failure (15864); OAuth skips duplicate prompt param (16424).
- Missing list item error tells agent how to resend (16245); download failure says check connection (16354).
- Keep full MCP URL with token instead of moving to header (16362); Windows non-ASCII path for npx/node (16234).
- Gemini drops an app tool with an untyped list field, naming it (16393).
- Cursor empty model list retried once then a plain hint (16062); OpenCode Go usage ring for sign-in key (16230).
- Group hidden/superseded ChatGPT models under "Older models" (15268); same-email accounts numbered (15340).
- "Paste the address" OAuth fallback for stalled Gemini sign-in (13892).
- Computer Use unready names the missing permission (15424); slow local detection fields persisted (12816).
- Usage: "Using credits" and Reset usage already done (208).

### Skills / vault / import
- Adding a marketplace skill already in the folder resolves by id with a rename/delete hint (15339).
- Stop re-asking to save a skill at the 32-skill cap (15507).
- Settings help: Vault, Import, "Use my Claude Code setup", account/provider search (15947/16129/16151/15623/15922).
- Typed folder path allowed in folder picker so hidden folders can be granted (15342).
- Timed-out agent delete unmarks the bot and doesn't block others (16179).

### Desktop / clients
- Workflow templates (5 + blank canvas) and runnable default model per step (bf50ae89) — also A.
- Roster tabs All/Unread/folders with unread badges (t3-bot-tags-tabs).
- Links clarify in-app vs default browser (12514); splash/workspace shown while loading (splash-min-shown, 15116).
- Text selection in table cells gets Quote/Explain/Improve/Describe (15429); same for selections with links (16114).
- Quote/Explain/Improve routed to originating side thread (15134).
- Agent opens a page → composer notice with Open page (14719).
- Attach files and accept drops on side threads (14867/15213/15224).
- MCP connection error wraps over multiple lines (15785).
- Island: working bot's name/picture opens that bot (15468); import window keeps open with exact reason (16295).
- "Add account with code" for second Gemini account (16306); keep focus in composer after Send (16360).
- Taller Mermaid fades at bottom with hover scroll arrow (16265); spreadsheet too large → open/download (16021).
- Bold/strikethrough after CJK punctuation (7015); HTTP Basic auth sign-in box over address bar (8462).
- Connection circles full size at 50% zoom, 2× at 25% (15136).
- Rename opens the name field (15099); exclude in-flight message from queued count (14893).
- Search-animation settings row, Android branch button, live cursor hookup, visual-reply card placement — deferred items from the full inventory, now in scope.
- Voice language retry (16083) and linked-workspace skills (16178) — deferred, now in scope.

## Not portable (with reason)

Joined servers (member rejoin, per-server sign-ins, channel read cost, idle Gemini notices, server-list scroll, version gates), hosted/account service (site storage, invite codes, credits), web client, Chrome extension/X, OS packaging, performance-only internals, agent dock separate window, Muse/Windows credential stores, MacBook notch.
