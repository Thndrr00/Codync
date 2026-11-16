# Subagents

A turn's subagents appear as one card of their own, with the task each was given, its state, the
step it is on and its time. While the reader scrolls away from a card whose subagents still work, its
header stays under the chat header with the progress, the current step and the time; it takes no
height and a click goes back to the card.

## What the card shows

The host writes one `subagents` entry per turn, with this data (all clients render it):

```json
{
  "author": "<bot id>",
  "text": "Subagents: Review the upload API, Fix the retry",
  "subagents": [
    {
      "id": "<tool call id>",
      "task": "Review the upload API",
      "state": "running",
      "step": "Read src/api/bots.rs",
      "startedAt": 1760000000000,
      "endedAt": null
    }
  ]
}
```

`state` is `running`, `done`, `failed` or `stopped`; times are epoch milliseconds; `endedAt` is null
while the subagent works. `text` is what a client that does not know the card falls back to. The list
is the whole list each time: the entry updates in place, as a plan does, so a turn keeps one card
even when more subagents start. Older clients show the entry's text as a notice.

## What drives it

- Managed agents run over ACP, which reports a subagent as the tool call that started it: Claude
  Code's `Task` and `Agent` calls, Codex's `spawnAgent`, and Codync's own team tools `ask_bot`,
  `delegate_batch` and `run_pipeline`. `chat::subagents::task_of` maps those names and their
  arguments (a `description`, a `prompt`, a batch's first goal, `ask_bot`'s message) onto the card's
  task; the call's later title is its current step, and its `completed`/`failed` state ends the
  subagent. A turn that ends ends the subagents it started.
- A terminal session (`host/src/external`) says the same with Claude Code's `SubagentStart` and
  `SubagentStop` hooks, which Codync installs beside the others; its `Task`/`Agent` `PreToolUse`
  names the task, and a stop's last message is the final step.
- ACP does not report what happens *inside* a subagent, so a card shows the task, the state, the
  newest step the call named and the time. It does not unfold a subagent's own tool calls.

## Clients

- Desktop: `views/thread/Subagents.tsx` (the card and the dock, mounted by `ThreadView`), the card
  from `ChatRows.tsx`, and `subagent-card.ts` for the newest working card and the time.
- iPhone: `Thread/SubagentCard.swift`, opened from `ChatRows.swift`; `ConversationList` reports the
  rows on screen and takes a row to scroll to, so the dock knows when the card is away.
- Android: `SubagentRows.kt`; the dock reads `LazyListState.layoutInfo` and scrolls with
  `animateScrollToItem`.
- Terminal UI: `view/transcript.rs` draws the card and `view/chat.rs` paints its header on the first
  transcript line when the card is out of view; `Click::Subagents` scrolls back to it.

All four ease the dock in with their own motion constants and keep it out of the layout, so the
messages never move when it appears. With Reduce Motion on, it appears and disappears at once.
