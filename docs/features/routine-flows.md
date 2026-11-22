# Routine flows

A routine can pass its result on. When the routine's bot answers, a **link** hands that answer to
another bot, with an instruction of its own ("summarize this", "translate it"), and that bot's
answer can go on to a third. A flow lives *inside one routine*: the routine's own bot starts it,
and each bot that takes part in a run leaves a **step**. The host runs it (`host/src/routines/flow/`);
the clients only show it and send intents. The design is ported from Openstark's routine flows
(`routine-flows-v1`) with Codync's vocabulary (bot, not agent); no code was copied.

Flows are built by talking to the bot ("when the digest is ready, give it to Ada to translate"),
from the graphical canvas of the desktop and phone apps, or in the terminal UI (`f` on a routine).

## Model

| Thing | Fields |
|---|---|
| **Link** | `id`, `routineId`, `fromBotId`, `toBotId`, `name` (≤ 200 chars, empty = no name), `instruction` (≤ 8000 bytes, may be empty), `model` (`null` = the target bot's own), `modelOptions` (that model's settings, e.g. `effort`), `createdAt` (ms) |
| **Step** | `id`, `runId`, `botId`, `deliveryId` (the bot request that carried the input; `null` for the routine's own bot), `input`, `output` (`null` until it replies), `status`, `error`, `files` (the pictures or clips a media step made), `createdAt`, `updatedAt` |
| **Step status** | `running`, `succeeded`, `failed`, `skipped`, `cancelled` |
| **Media step** | `id`, `routineId`, `name` (≤ 200 chars, empty = no name), `kind` (`image`/`video`), `model`, `prompt`, `aspect`, `duration`, `resolution`, `voice`, `transparent`, `createdAt`, `updatedAt`; node key `media:<id>` (see [media](media.md#routine-flow-steps)) |
| **Position** | `nodeKey` (`routine:<id>`, `bot:<id>` or `media:<id>`), `x`, `y` (numbers, at most ±1 000 000) |
| **Group** | `id`, `name` (≤ 120 chars), `members` (node keys, each in at most one group), `collapsed` (folded into a stack card), `x`, `y` (card position) — drawing only, never changes what runs |

A **canvas** belongs to a bot and holds every routine it owns or that passes through it, the links of
those routines, where the nodes sit, the bots **placed** on it before any link reaches them, and the
**groups** of nodes drawn together. Groups are ported from OpenBot 0.74.0 but use Codync's model:
a group bundles canvas nodes, it is not a workflow of its own.

## Grouping nodes

Select several nodes (click with Ctrl/Cmd or Shift held to add them), press **Group** (Ctrl/Cmd+G) and
give the group a name. A group has **one port per distinct output kind** of its members — a bot's
*Reply*, a picture step's *Picture*, a clip step's *Clip*. A line drawn from one of those ports
reaches every member of that kind. Links from several members to the same node are drawn as **one line
with a count**, not a tangle of lines. Drag a group card to move its members with it.

Press **Collapse group** (or double-click its card) to fold the members into a **stack card** that
previews the pictures it holds; double-click the card to open it again. **Ungroup** (Shift+Ctrl/Cmd+G)
ends the group but leaves every node and link in place — a group never changes what a routine runs.
Pasting or dropping two or more files on the canvas makes one picture step per file, all in a single
new group. An older host or canvas with no groups simply shows the nodes ungrouped.

On the desktop each open workflow is a **tab** in the flow view's title bar, ported from OpenBot nightly's workflow tab mode. Tabs reuse the shared `TabStrip`: click to open, double-click to rename the workflow inline, drag to reorder, and middle-click or the tab's menu to close it, its siblings or every tab to its right. The open tab list and order live in the device pref `openWorkflowTabs` (workflow ids only); a tab whose bot is working, or whose canvas has a step running, shows its running state.

The **All workflows** menu lists every workflow with its folder and can make, rename or delete a folder and move a workflow into a folder or out of it. Folders belong to the computer, not the device (the host owns them): `workflowFolders` reads `{revision, folders: [{id, name}], placement: {workflowId: folderId}}` and `workflowFolderApply` takes one `{action}` body — `create` (`id`, `name`, optional `workflowIds`), `rename` (`folderId`, `name`), `delete` (`folderId`) or `place` (`workflowIds`, `folderId: null` takes a workflow out of its folder). An older host without these methods simply shows every workflow unfiled. Folders are navigation only — a flow's links and steps never change.

The host keeps that snapshot in one bounded kv blob (`workflowFolders`) stamped with the global rev, so every computer client reads the same tree and catches up when it moves. It enforces at most 200 folders with unique names of at most 80 characters, 10,000 placements and 1,000 workflows in one action, checks ids, names and action keys strictly, and drops the placement of any workflow deleted from the routine registry on load and before an apply. A `create` may file its `workflowIds` as it makes the folder, `place` moves several workflows at once (a `null` folder files them in "All workflows"), and a `delete` removes that folder's placements too. The optional `folders` field of `routineFlowCanvas` is omitted on an older canvas with no folder blob.

A video step's **Length** menu lists the lengths its model declares. When the model declares more
than one, **Custom** opens a slider over the range those lengths span (the smallest to the largest,
one second per step), so the step can make a clip of exactly that length. The value is saved with the
step; a host that doesn't know Custom keeps the short list.

## Collaborative edits

When two people edit the same canvas, the host merges their changes field by field instead of letting the last save win: a save carries the revision the person started from, the host finds that document in a short stamped history (`routineflow.docs.<botId>`), and a pure three-way merge (`host/src/routines/flow/merge.rs`) folds the nodes, links, step settings and groups together. An entry only one person changed is always kept; two different versions of one entry — or a change against a removal — are reported as conflicts with both versions, and `saveRoutineFlow` stores the merge as pending until `resolveRoutineFlowConflict` is called once per choice with `local` or `incoming` (the merged groups are written to the canvas in the same transaction). A person editing alone sees no difference, an edit that folded in quietly shows "someone else edited this", and an older canvas whose revision is no longer in the history is saved as-is with no base. A merged save also writes the merged positions, links and media steps back to the runnable flow, so the next canvas reload shows them under the same rev as the groups. A conflict nobody chooses within
seven days is dropped on the next load along with its snapshots, so an abandoned merge cannot hold a canvas
to a stale base; resolving a conflict saves it again and restarts that window. Saving the canvas's groups
reports `routine_flow_grouped` (no document data) — see [analytics events](../reference/analytics-events.md).

## Running a flow

A routine's card on the canvas shows **Run** while it has no run in flight and **Stop** while one is
queued or running: the host puts the live run's id in the routine's `running` field
(`Routines::list`), and Stop cancels the bot's turn, which is the run's turn. Ported from `OpenBot`
0.71.0, where Stop takes the place of Run now in the canvas top bar.

1. The routine's run ends (`Routines::complete`). A reply other than `(pass)` becomes the root step
   (`output` = the reply, `botId` = the routine's bot). A failed, interrupted or cancelled run records
   the root step that way and **skips** every bot the flow reaches.
2. Level by level (depth 1, 2, …) each bot of the flow gets a request through the **same road as
   `ask_bot`** (`chat/team/flow.rs`): the bot's actor queue, an exchange notice in its chat ("Routine flow ·
   name · from Ada"), the assignment ledger and the reply. The prompt carries the replies of the bots linked to
   it and the links' instructions. A flow needs no turn of the sending bot (the host sends it).
3. A bot with several inputs (a join) waits for all of them and gets them in one request. If some inputs
   failed, it runs with the ones that replied; if none replied, its step is `skipped`. A bot that replies
   `(pass)` succeeds with no output, so what hangs below it is skipped.
4. Each step has the **routine's run timeout**. A step that fails leaves its bot's step `failed` with the
   error, and everything that only depended on it is `skipped`.

Rules the host enforces:

- **No cycles**: connecting refuses a link that would close a loop, and running checks again (a loop in
  stored links skips the whole flow and logs an error).
- **Depth**: at most 5 hops from the routine's own bot; set `CODYNC_ROUTINE_FLOW_MAX_DEPTH` (1 to 16) in the
  host's environment to change it. Connecting counts what already hangs below the new link; running skips bots
  deeper than the limit.
- **Targets**: only a bot that takes requests like any other: not a group, not a terminal session
  (`external::is_external`), not hidden or deleted. The same check applies when connecting, when placing a bot and when running.
- **Media steps** are link ends like bots: a picture can be handed on, and a bot's reply can be handed
  to a step. A step's settings are checked against its model before it runs (frame, length, resolution,
  voice, transparency, and the backend that makes the kind). A step whose model id starts with
  `sunra-` runs through the installed, signed-in Sunra connector instead of an agent: the host spends
  that sign-in's credits, polls the job, and keeps the file like any other media step. Without the
  connector the step fails with a sign-in message; flows that use other models are unchanged. See
  [media](media.md#sunra) for the model list, credits and transfer rules.
- **Connecting rules** (`graph.rs`): not to itself, never into the routine's own bot, only from a bot the flow
  already reaches, no duplicate link between the same two bots, no loop, no deeper than the limit.
- **A link applies to runs that began after it was made.**
- **Idempotent**: a bot has at most one step per run (`UNIQUE(run_id, bot_id)`), so running the same run again
  adds no steps and sends nothing twice. Only the newest 50 runs of a routine keep their steps.
- **Cancelling**: deleting the routine, deleting the bot, disconnecting a link or the owner bot's *Stop* cancels the
  routine's running flows. The step in flight ends `cancelled` and the rest are skipped.
- **Restart**: steps still `running` when the host starts fail with "The host restarted while this step was running",
  and the bots of a cut-short flow that never got a step are `skipped` (`recover.rs`). The flow is not resumed.
- **Disconnecting** a link does not remove links that hang below it; they stay but are not reached until the
  flow reaches them again.

A step's menu can start a **run from this step**: the host begins a new run there, recording the stored answers
and settings of the steps before it (and reusing picture answers as given) instead of asking those bots again; it
re-runs only the chosen step and what hangs below it. Each step now also keeps the prompt, model, options and
inputs it actually ran with, so **Settings** shows what made a result and **Use these settings** puts them back
into the link or picture step for a rerun (older runs simply show no saved settings). A bot step shows its agent's
model as text — set a model on the link for a different one, or add a picture/clip step for generated media — and
pictures connected to a step are handed on with an instruction to use them as given, or to stop with a clear
message if the model cannot read pictures. A schedule trigger adds its run time in UTC and its zone to the steps
that follow so a model can compute a time window. Duplicating a picture step keeps its settings and the links
that fed it, and selects and raises the copy; an empty canvas shows a sample workflow and **New workflow**.

## Reading and naming steps

Select a step and click its title to give it a **name** (Enter or blur saves, Escape cancels). The name is
stored on the link that reaches a bot step, or on the media node itself, with `#[serde(default)]`: an older
flow has no name and reads as unnamed. It shows on the card, in the run status list, in canvas search and in
`list_routine_flow`, and it is used when a step is named in the text handed to the next bot. An empty name falls
back to the step type. On the desktop this is a small inline editor on the card; the host method
`renameRoutineFlowStep` is the only writer.

While a bot step runs, its card shows the bot's live **activity**, the step before it and how many steps are
done. These come from the bot's status/activity and the run's step statuses already sent to clients — no new
polling — and the card reserves a fixed height so changing text never resizes it. Selecting a picture or clip
step opens a **composer** under its card with the description, the pictures connected to it, the model, its
settings and **Run** in one place; it reuses the inspector's shared controls and the saved step settings.

A card's output **port** keeps a soft tint of what its visible line carries — text, a picture or a file — using
the theme's own semantic colour tokens; a port with no line stays grey. A step's result shows the **newest file**
as soon as a run ends; round thumbnails (with a thin ring coloured from the result and a blurred backing) walk
the older files of that step, and the arrows do the same. A new run starts on its newest file again,
so a file clicked earlier is never left showing as the current result. A result still being made shows a spinner,
not an empty result.

Other fixes from the same nightly apply to the desktop canvas: files pasted or dropped into a flow land in the
middle of the visible canvas instead of a far-off saved position; the empty-workflow message floats over a live
canvas rather than replacing it, so drawing, pan and paste clicks pass through; and a click on the canvas moves
focus there, so Backspace deletes the selected steps instead of acting on the title-bar tab. Opening another
workflow shows a spinner until its canvas arrives, so an open canvas is never mistaken for an empty one.

## Host API

All methods use the usual authenticated API (`POST /api/<method>`) and the E2E channel with a device that has the
`control` scope: the same access as `routines`. Changes are `rev`-stamped and reach every client through events.
Request bodies are JSON objects; ids are 1 to 200 characters; unknown or invalid input answers `400` with
`{"error": "..."}`.

| Method | Body | Answer |
|---|---|---|
| `routineFlowCanvas` | `botId` | Canvas (below) |
| `saveRoutineFlowPosition` | `botId`, `nodeKey`, `x`, `y` (node must exist) | `{"ok": true}` |
| `removeRoutineFlowPosition` | `botId`, `nodeKey` (a `bot:` key also unplaces the bot) | `{"ok": true}` |
| `placeRoutineFlowBot` | `botId`, `placedBotId`, `x`, `y` | `{"ok": true}` |
| `connectRoutineFlow` | `routineId`, `fromBotId`, `toBotId`, optional `instruction` | `{"link": Link}` |
| `disconnectRoutineFlow` | `linkId` | `{"ok": true}` |
| `updateRoutineFlowLink` | `linkId`, `instruction`, optional `model` and `modelOptions` (absent keeps, `null`/`{}` clears) | `{"link": Link}` |
| `addRoutineFlowMedia` | `routineId`, `kind` (`image`/`video`) | `{"media": MediaStep}` |
| `updateRoutineFlowMedia` | `mediaId` and any of `prompt`, `model`, `aspect`, `duration`, `resolution`, `voice`, `transparent` | `{"media": MediaStep}` |
| `removeRoutineFlowMedia` | `mediaId` | `{"ok": true}` |
| `saveRoutineFlowGroups` | `botId`, `groups` (the whole list of Groups; every member must name an existing node) | `{"ok": true}` |
| `saveRoutineFlow` | `botId`, `baseRev`, `document` (`nodes`, `links`, `settings`, `groups` as id → version maps, optional `rev`) | `{"saved": document, "merged": bool}` or `{"conflicts": [...], "document": merged}` |
| `resolveRoutineFlowConflict` | `botId`, `field`, `key`, `choice` (`local`/`incoming`) | `{"saved": document}` or the remaining `{"conflicts": [...], "document": merged}` |
| `testRoutineFlow` | `botId` (the routine's owner), `routineId`, optional `input` | `{"runId": "flow-test-…"}` |
| `runRoutineFlowFromStep` | `botId`, `routineId`, `nodeKey` (`bot:` or `media:`) | `{"runId": "flow-step-…"}` |
| `renameRoutineFlowStep` | `botId`, `routineId`, `nodeKey` (`bot:` or `media:`), `name` (≤ 200 chars, empty clears) | `{"ok": true}` |
| `workflowFolders` | — | `{revision, folders: [{id, name}], placement: {workflowId: folderId}}` |
| `workflowFolderApply` | `action` (`create` with `id`/`name`/optional `workflowIds`; `rename` with `folderId`/`name`; `delete` with `folderId`; `place` with `workflowIds`/`folderId`, `null` = out of folder) | the new snapshot |

`testRoutineFlow` doesn't run the routine: it starts the flow as if the routine's bot had just replied with `input`
(a sample sentence by default), so a link can be tried without spending a run. Its steps carry the returned
`runId`. Run the routine itself with `runRoutine`.

**Canvas** (`routineFlowCanvas`):

```json
{
  "botId": "…", "rev": 123,
  "routines": [{
    "routine": { "id": "…", "botId": "…", "name": "…", "instruction": "…", "triggers": [], "enabled": true,
                 "timeoutSeconds": 3600, "lastError": null, "triggerDescriptions": [], "nextRunAt": 1730000000000,
                 "createdAt": 0, "updatedAt": 0 },
    "recentRuns": [ /* the newest 10 routine runs, newest first, as in `routines` */ ],
    "upcomingRuns": [1730000000000],
    "steps": [ { /* … */, "files": [ { "id": "…", "name": "…", "size": 0, "sha256": "…" } ] } ],
    "stepSettings": { /* step id → the settings and inputs it ran with; absent for older runs */ }
  }],
  "links": [ { "id": "…", "routineId": "…", "fromBotId": "…", "toBotId": "…", "name": "", "instruction": "", "model": null, "modelOptions": {}, "createdAt": 0 } ],
  "mediaNodes": [ { "id": "…", "routineId": "…", "name": "", "kind": "image", "model": "gpt-image", "prompt": "…", "aspect": "1:1", "duration": 6, "resolution": "480p", "voice": null, "transparent": false, "createdAt": 0, "updatedAt": 0 } ],
  "positions": [ { "nodeKey": "routine:…", "x": 0.0, "y": 0.0 } ],
  "placedBotIds": ["…"],
  "groups": [ { "id": "g1", "name": "Pictures", "members": ["media:…", "media:…"], "collapsed": false, "x": 0.0, "y": 0.0 } ]
}
```

`upcomingRuns` are the next fire times in the next 7 days (at most 400; empty while paused or for a webhook or
watch routine). All times are Unix milliseconds.

**Event**: `{"type": "routineFlow", "rev": N, "botIds": ["…"]}` — the canvases of these bots changed (a link, a
step, a position, a placement); fetch `routineFlowCanvas` again. The event carries no data on purpose: a client
that doesn't know it can't be broken by a new field, and one that does always reloads. Catch-up (`/events?since=`)
holds one such event for every bot whose canvas changed after `since`, in rev order. Clients must never skip it.

## Bot tools

The built-in `routines` MCP server has three more tools, scoped to the bot's own routines:
`link_routine_bots` (`routineId`, `toBotId`, optional `fromBotId` and `instruction`), `unlink_routine_bots` (`linkId`,
or `routineId` + `toBotId`) and `list_routine_flow` (optional `routineId`), whose links and media steps carry each
step's `name` so an agent reading the flow sees the names. The server's instructions tell the bot when to use them
(see [Routines](routines.md#conversational-setup-and-tools)).

## Storage

SQLite tables `routine_flow_links`, `routine_flow_steps`, `routine_flow_positions` and `routine_flow_placed`
(`host/src/store/flows.rs`, `flow_steps.rs`), with indexes by routine, owner bot, target bot and step status.
A bot's groups live in the kv blob (`routineflow.groups.<botId>`, written and validated by
`host/src/routines/flow/groups.rs`), so an older canvas without the field reads as ungrouped.
Routines themselves live in the `routines.v1` JSON blob, so the cascade is explicit: deleting a routine
(`flow::forget_routine`) or a bot (`flow::forget_bot`, also at startup for deleted bots) removes its links,
steps, positions and placements in one transaction. Each change stamps a global `rev` and records it per bot in
`kv routineflow.rev.<botId>`.

## Compatibility

Only methods, an event type and fields were added: `minApp` and `minHost` do not change, and older apps and hosts
ignore the new method names and `routineFlow` events (the TUI and apps skip unknown event types by design).

## Screens

In each client, a bot's routines screen has a **List | Flow** selector. Flow shows the canvas of that bot: routine, bot and media nodes, a fixed dashed edge from each routine to its own bot, link edges, saved positions (nodes never moved sit in automatic columns by depth), and each bot's latest step status (icon and words, never colour alone). A **list** of the same flow is the keyboard and screen-reader equivalent (and the phone's default with TalkBack on or on narrow screens). Both open the same inspector: a routine (pause/resume, test, add a picture or clip step, latest steps, run history), a bot (hand its reply on to…, remove if placed by hand), a media step (description, model, frame and a video's length, resolution and voice, transparency, save, remove, hand its file on to…, which also hands on to bots) or a link (instruction, model and its options, disconnect). States: loading, no routines, not connected, no runs yet, error with retry, offline (read-only) and a host without the methods (the canvas says to update Codync there). The desktop rail's **Bot canvas** button opens the selected bot's flow in the same large sheet.

- **Desktop** (`apps/desktop/src/renderer/views/routines/flow/`, model `lib/routine-flow.ts`, state `store/routine-flow-store.ts`): hand-written pan/zoom/drag; connect by dragging from a bot's dot onto another bot, or from the inspector. Choosing **Flow** opens the canvas in a large sheet (the details column is too narrow for a graph); the same canvas can be reopened from its expand button. A routine card shows the next seven days as seven tick columns and its newest runs as status dots, with a play button that tests the flow (disabled until a bot is linked). Multi-select and Ctrl/Cmd+G group nodes (one port per output kind, counted lines, a collapse/expand stack card, Shift+Ctrl/Cmd+G to ungroup); a video step's Length menu has a Custom slider over the model's declared range. Groups are desktop-only drawing state: the phone and terminal clients keep showing the ungrouped nodes and links.
- **Android** (`RoutineFlow*.kt`, model `core/RoutineFlow.kt`): pan, pinch and node dragging; connect from the bot's detail sheet; the same week strip, run dots and play button on a routine card.
- **Terminal** (`f` on a routine): the chain as rows; same words (Running, Succeeded, Failed, Skipped, Cancelled).
- **iPhone**: not built yet, so routine flows and their media steps are read and edited on the other clients.

The clients pre-check the connect rules (`connectProblem`, same wording as the host) and the host stays the authority.

## Trying it

See [Testing routine flows](../guides/routine-flows-testing.md).
