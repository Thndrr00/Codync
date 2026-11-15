# Missions

A mission is a goal that outlives one turn. A routine finishes and is forgotten; a mission
remembers what was done, what is left and why it stopped, so the next turn resumes instead of
guessing. The idea comes from agentes.chat, as a pattern rather than ported code: their host is
TypeScript and AGPL-3.0-only, ours is Rust.

The host owns missions end to end: storage, the status rules, the limits, the push and the tools.
Every client only renders that state and sends the moves a person makes.

## The model

A mission has a bot, a title, a goal (why, and what counts as done), a status, a checkpoint, an
optional stop reason, an optional link to the routine run executing it and an optional link to the
approval blocking it. Its steps (`mission_steps`) carry a position, a text and their own status.
Nothing is deleted when a mission ends: it stays, with its steps and its reason, until a person
drops the history.

Statuses are `proposed`, `active`, `waiting`, `done` and `abandoned`. The allowed moves are:

| From                 | To                                  |
| -------------------- | ----------------------------------- |
| `proposed`, `done`   | `active`, `abandoned`               |
| `active`, `waiting`  | `active`, `waiting`, `done`, `abandoned` |
| `abandoned`          | `active`                            |

A move to the status a mission already has is allowed and leaves it where it is. Abandoning
requires the reason, so the next person knows why it stopped. A step is `pending`, `active`, `done`
or `skipped`, and a skipped or finished mission keeps its steps as they were.

Limits keep the board readable: titles are 200 characters and texts 4000, a mission carries at most
100 steps, a bot runs at most 3 active missions and the whole fleet 12. Only `active` counts, so
proposing is never blocked; activation is. When the host restarts, every mission that was running is
parked in `waiting` with the reason "the host restarted mid-run", its run link cleared, and pushed if
push is on.

## Where it shows

The desktop inspector, the terminal UI, the iPhone details sheet and the Android details sheet each
have a Missions page under the bot: open missions with their status, goal, steps and checkpoint, a
composer for a new one, and finished ones behind a toggle. All four offer the same actions: activate
or resume, pause, finish, abandon with a reason, write the checkpoint, and add, start, complete, skip
or reopen a step. Abandoning asks for the reason in the shared dialog.

A mission change is broadcast as a `mission` event, so every open page follows the same move. A
finish pushes "Mission complete" and a block (moving to `waiting`) pushes "Mission blocked" with the
stop reason, both through the bot's own notification path.

## For bots

Every agent receives the `missions` MCP server: `missions` lists its own open missions (or one by
id, or all with `all`) and `update_mission` moves one along — a status, the stop reason, the
checkpoint, the run or approval link, or a step with `text`, `stepId` and `stepStatus`. A bot reaches
only its own missions. The tool tells it to write the checkpoint whenever it stops, because the next
turn starts from it.

## API

`createMission`, `missions` (with `botId`, `query` and `open`), `mission`, `updateMission` and
`missionStep`. Search covers titles; `open` hides everything already returned.
