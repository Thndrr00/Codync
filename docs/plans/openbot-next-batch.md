# Next integration batch (24 additions)

Source: OpenBot nightly backend/agent features not yet in Codync. Rule: reuse Codync's machinery,
never a second store or agent loop; add a short source comment; docs in the same change.

## Host behavior (batch A — agent tools, approvals, idempotency)
1. `interrupt_bot` tool: stop another bot's turn with a reason, reusing `hub.stop_bot` and the team tools.
2. Auto-approve automation preference per agent (`setApprovalAutomation`), reusing the approval card.
3. Tool-call idempotency: dedupe repeated calls with the same key inside a turn (mirrors `clientNonce`).
4. Plan continuation: treat a short "continue"/"keep going" as continuing unfinished work.
5. Quiet routine runs: keep steps but post nothing.
6. Routine start retry once on a transient failure.
7. Turn traces: record steps, memory changes and subagents per turn, expose in run status.
8. Delete-for-everyone + edited-message text on channel messages.

## Host messaging and data (batch B)
9. Side question with a bounded transcript of earlier chat (reuse memory/history tools).
10. Attention request lifecycle: expire stale attention cards (reuse notification state).
11. Image and video reader tools for agents (read a stored file's metadata/content).
12. Commit co-author tool (`Co-authored-by:`), reusing shell commands.

## Desktop (batch C)
13. Visual page preview in chat (sandboxed, no scripts).
14. Tool-guidance tooltips on approval cards (why a tool is risky).
15. Inline image/video viewer improvements (zoom, fit).
16. Command palette: recent and grouped actions.
17. Library storage usage panel per bot (reuse file sizes).
18. MCP tool policy editor UI (allow/ask/deny per tool).

## Mobile (batch D)
19. iOS/Android inline media viewer with zoom.
20. iOS/Android unread/attention badges in roster.
21. iOS/Android send files from photos (reuse attachment path).
22. iOS/Android message-context jump parity.

## Cross-cutting (batch E)
23. Analytics events for the new tools and actions (fixed properties only).
24. Docs: one page per subsystem + pointers from docs/README.md.

Not portable (no Codync equivalent): CRM, hosted servers, web client, joined-server protocols,
invite credits, Signal regions, the dock's separate OS window (we integrated it into the island).
