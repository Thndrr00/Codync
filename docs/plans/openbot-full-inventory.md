# Full OpenBot → Codync inventory (November 2026)

The user asked for ALL of OpenBot, not just version changelogs. Three read-only audits covered:
- the 21 new fragments on `origin/main@33f7406e3` (post-v0.78.0),
- all 19 files under OpenBot `plans/`,
- the architecture/feature docs under OpenBot `docs/`.

Rule: copy behavior with a short source comment, reuse Codync's code/APIs/components, pure tested
decisions wired to real call sites. Excluded with reasons: joined servers, hosted/account service,
web client, Chrome extension/X, OS packaging, performance-only internals. 39 portable items below.

## State

All portable items are integrated in six disjoint batches and pushed (commits 204-209); the catalogs
carry 17 new labels. A few items are partial where the real call site lies outside the batch scope:
voice language retry, symlinked-workspace skills, the "Animate search results" settings row, the
Android branch button, Computer Use live cursor hookup and the visual-reply card placement are
documented with reasons in their batch docs. Nothing here was compiled or run end to end.

## From the 21 new fragments

1. Subagent helpers use the parent's browser/own tools (15942).
2. WhatsApp phone→allowed-contact messages saved in history, no new turn (15977).
3. OpenCode Go uses the saved key over an old env key; clear "own key" error (15988).
4. Telegram Reply shows the quoted message / selected part to the agent (16000).
5. Empty folder inside an installed skill is not a local change (16051).
6. First voice reply read aloud again when language undetectable (16083).
7. HTML `#` links in chat preview open their part of the page (16087).
8. Codex sandbox-start failure says "choose Ask first or Full access" (16118).
9. Skills install on a workspace folder that is a symlink, refusing inner links (16178).
10. A Gemini turn silent with an open step for 30 min ends and frees the queue (15950, partial).

## From OpenBot plans

11. MCP failure toast for every dropped server, deduped by cause (003).
12. MCP panel note about the working-directory limit (003, low impact).
13. "Animate search results" setting with blur/fade/none and reduced motion (004).
14. Delay the Searching spinner by 200 ms (004).
15. Keep prior search hits while a query loads (004).
16. Enter/exit/move motion for search rows and sections (004).
17. Branch from here only on the last message of a finished turn, disabled while running (010).
18. Idempotent branch request with a client nonce (010).
19. Auto-title a branch and nest it under its source (010).
20. Managed runtime provisioning for npm-based MCP servers (003, uncertain packaging).

## From OpenBot docs

21. Local scripts API + bundled CLI with per-agent opt-in, wait/answer, run/awake routine (automation.md).
22. Automatic cookie-banner refusal in the agent's browser (browser.md).
23. CAPTCHA/bot-check awareness in snapshots with a `challenge` field and takeover (browser.md).
24. Site-scoped memories injected into a browser snapshot (browser.md).
25. Messaging connection memories (`messaging_remember`/`forget_memory`) (messaging.md).
26. `messaging_post` tool to post to a connected place from a chat/routine (messaging.md).
27. Webhook routines accept the sender's own Standard Webhooks secret (webhooks.md).
28. Signed "Send test event" and ignored-delivery history (webhooks.md).
29. Reset-usage credits action and "Using credits" composer state (providers.md).
30. Visible agent cursor overlay for Computer Use (computer-use.md).
31. `html_render` interactive visual reply published above a reply (agents.md).
32. `set_backdrop` agent-set window backdrop with Undo (agents.md).
33. Quote-selected-text chip on a published HTML page (agents.md).
34. Site memory shown in the memory editor and client (browser.md).
35. Proactive posting tools in the agent's own chat from a routine (messaging.md).
36. MCP server start-up failure notice generalized to all providers (003/15811).
37. Branch request id in the desktop client as well as Android (010).
38. Agent finds Vault/Import/Claude setup help via a settings catalog (15947/16129/16151, partial).
39. Test event signer exposed through the webhook routine editor (webhooks.md, UI half of 28).

## Not portable (with reason)

Joined servers (rejoin member, per-server sign-ins, channel read cost, idle Gemini notices),
hosted/account service (site storage, invite codes), web client, Chrome extension/X, OS packaging,
performance-only internals. No settings-lookup tool exists for item 38's full form, so only the
bounded help entries are portable.
