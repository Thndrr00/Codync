# Chat history actions

A chat's or a group's settings have three history actions, ported from OpenBot 0.65. They are host
methods, so every client calls the same ones and the terminal UI offers them as keys.

| Action | Method | What it does |
|---|---|---|
| Delete history | `deleteHistory {botId}` | Deletes the chat's messages and turns (`thread_id IS NULL`) and stops its running work. The bot or group row, its setup, memories, routines, artifacts and files stay; side threads and every other chat keep their lines. Typed confirmation on the apps: type the chat's name. |
| New conversation | `newSession {botId}` (the existing method) | Keeps the messages and starts the agents fresh. An agent's session is forgotten (`Cmd::NewSession`); a group posts a divider and records `conversation.start.<group>`, so members see only the room lines after it. |
| Compact context | `compactContext {botId}` | Queues `Cmd::CompactNow` for the agent, or for every member of a group. Only for an agent Codync can compact (Claude, Codex, a custom agent with a known window); otherwise the host refuses. |

## Scope and limits

- `deleteHistory` is implemented in `store/history.rs` (`clear_history`) and `hub/history.rs`. The
  deletion is one transaction; `historyCleared {botId, deleted, rev}` tells clients to drop their
  copy. The desktop, iPhone, Android and terminal UI all handle it.
- A group's members share one conversation across all their chats, so `compactContext` on a group
  also compacts each member's own chat. The apps say so.
- A terminal-session bot (`external::is_external`) keeps its own CLI conversation: `newSession` is a
  no-op and `compactContext` refuses, because Codync cannot start a session or compact inside it.
- Nothing touches memories, routines, artifacts, files, side threads or the bot/group row.

## Keys

- Desktop: the chat's Settings tab (or a group's editor) has a **History** section.
- iPhone/Android: the same section in a bot's settings and a group's editor.
- Terminal UI: `X` deletes the history after a confirmation, `Z` compacts now, and `S` starts a new
  session (also for a group).
