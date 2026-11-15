# Brains: memory per project

Status (2026-11-15): built on 2.52.0, on every client.

A **brain** is memory several bots share: one project's, or the **Fleet** brain, which stands over the memory all
bots used to share. A bot works in one brain at a time, or in none; without one it keeps only its own memory.

The idea and the word come from [agentes.chat](../features/agentes-chat.md) (ADR 0032), where a *brain* names the
engine behind an agent and only the shared-scope idea carries over. The code is ours.

## Scopes

Three, in the order a bot reads them:

| Scope | Folder | Kept under | Cap |
|---|---|---|---|
| Personal | `<data>/bots/<id>/bot-memory/` | the bot's id | 1,200 tokens |
| Brain | `<data>/brains/<id>/` | `brain:<id>` | 1,200 per bot, by the brain's cap |
| Fleet | `<data>/shared-memory/` | `''` | 400 tokens, the old "yours" cap |

A bot sees its personal memory and its brain's, never another project's. The Fleet brain *is* the old shared layer:
`shared_memory` still works and still broadcasts, and joining Fleet is the same as turning shared memory on
(`brain: null, sharedMemory: true`). Leaving keeps only the bot's own memory (`brain: null, sharedMemory: false`),
picking a brain again brings sharing back, and a bot without a brain reads just its own.

The personal and brain layers reach only the bot; the Fleet layer is shared by every bot that joins it.

## Why not a fourth tier

agentes.chat's ADRs describe brains as an engine. Two scopes per bot already cover the useful cases: a bot's own
memory, and memory kept apart from every other context. A third tier (team, machine) waits until a real case shows
up.

## Host API

| Method | Body | Answers |
|---|---|---|
| `brains` | — | `{brains: [{id, name, shared, members}]}`, the Fleet one first |
| `createBrain` | `{name}` | `{brain, brains}` |
| `renameBrain` | `{id, name}` | `{brain, brains}` |
| `joinBrain` | `{botId, brainId}` | `{bot, brains}`; joining Fleet means `brain: null, sharedMemory: true` |
| `leaveBrain` | `{botId}` | `{bot, brains}`; out of every brain, own memory only |
| `deleteBrain` | `{id}` | `{bots, brains}`; its bots go to Fleet, what it held goes |
| `exportBrain` | `{id}` | `{format, version, brain: {name}, files: [{name, content}]}` |
| `importBrain` | `{bundle, name?}` | `{brain, brains, skipped}` |

Names are trimmed, at most 60 characters and unique without regard to case; a second one gets `(2)`, `(3)`… on
import. The Fleet brain cannot be deleted. Deleting a brain takes out its files, their versions, the facts'
provenance, the dream log of the layer and the marks of what the user had forgotten there, so nothing stale is
indexed or read again.

`memory`, `memoryFile` and the dream log keep answering `shared` and `bot` keys beside the new shape: `shared` is
still "this bot reads a shared layer" and an unseen scope reads as `personal` for an older client. The memory view
gains `brain` and `total {used, cap}`, and each dream line and each file change names the `scope` instead of a
`shared` flag.

## Files and search

Files live under the scope's folder, the bot's tools see the folders of the scopes it reads, and the memory
search indexes a file as `layer/name` (`''` is the Fleet layer) with the row's `scope` and `shared` saying which
layer a hit belongs to. A bot searches its own and its brain's memory only.

## Dreaming and compaction

Each scope compacts and dreams on its own. A dream run reads the layer it is about, and its changes carry the
scope's evidence, so a project brain's tidying never touches the person's lines. The Fleet brain keeps the
400-token "yours" cap; a project brain gets the bot's 1,200, since it holds one project.

## Screens

The brain row sits above "About you" and names the brain, with the total used against both caps. A sheet lists
Fleet and the projects and offers pick, create, rename, delete, export to `<name>.brain.json` and import from
such a file.

| Client | Where |
|---|---|
| Desktop | Memory → the row over "About you" → **Change brain**; export saves the file, import pastes one |
| Terminal | the memory screen: `b` opens the brains (↑↓ move, ↵ pick, `n` new, `r` rename, `d` delete, `x` save to a file, `i` import) |
| Android | the same row and sheet in the memory card; export opens the share sheet, import reads the picked file |
| iPhone | the same row and sheet in the memory card; export opens the share sheet, import reads the picked file |

## Tests

- resolution of a bot's scopes, and isolation: a brain's memory is found by its bots and by no other bot;
- joining Fleet leaves the project brain, leaving clears it, deleting sends the bots back to Fleet and keeps Fleet;
- export then import round-trips a brain under a free name and reports the files the checks turned away;
- the schema's migration to `brains`, and the layer comparison `layer(Scope::Brain)`.

## Not built

- A shared brain the user does not control, or one per computer.
- Merging two brains, or moving memory from one to another.
- Import writing straight into an existing brain (it makes a new one).
- A brain's own model or preset, as agentes.chat's word suggests.
