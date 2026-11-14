# Declarative UI and shared view state

Cards K3 and K4 of the CopilotKit survey. Both are implemented from their public behaviour: Codync
has no runtime dependency on A2UI, CopilotKit or `@a2ui/*` packages.

## Contrast: OpenIntelligentUI (card K14; reference only, no dependency)

Its generative-UI model lets a model compose arbitrary layout at run time: better for one-off,
exploratory boards, worse for Codync's guarantees. An arbitrary tree has no validated schema, so the
host cannot bound what renders, what it costs or which actions a page declares, and two agents can
produce visually inconsistent cards. Codync keeps these validated blocks and the fixed routine card,
where both agents render through the same schema and the same renderer. See
[routine flows](routine-flows.md#shared-cards-and-lanes-card-k14) for the multi-agent canvas.

| Card | Where | What |
| --- | --- | --- |
| K3 | `host/src/declarative/` | A bot shows a form, an approval, a table or a chart; the host checks it against Codync's own catalogue and posts it as an approval card. |
| K4 | `host/src/views/` | A small versioned state document per view (`chat:<botId>`, `routine:<botId>`), JSON-patch-like deltas from the apps, and named actions the agent invokes. |

## K3: declarative blocks

### The schema (version 1)

The host owns the catalogue; clients only render what it contains. A block is a JSON object:

```json
{
  "version": 1,
  "title": "Book the trip",
  "body": [
    {"type": "heading", "text": "Madrid"},
    {"type": "text", "text": "Two nights, from Tuesday."},
    {"type": "callout", "tone": "warning", "text": "The price may change."},
    {"type": "field", "field": {
      "id": "nights", "label": "Nights", "type": "number", "required": true, "value": 2
    }},
    {"type": "field", "field": {
      "id": "room", "label": "Room", "type": "select", "required": true,
      "options": [{"value": "single", "label": "Single"}, {"value": "double", "label": "Double"}]
    }},
    {"type": "divider"},
    {"type": "table", "columns": ["Item", "Price"], "rows": [["Room", 120]]},
    {"type": "chart", "chartType": "bar", "labels": ["Mon", "Tue"], "values": [2, 5]}
  ],
  "actions": [
    {"id": "approve", "label": "Approve", "kind": "primary"},
    {"id": "deny", "label": "Deny", "kind": "destructive"}
  ]
}
```

- **Components:** `heading {text}`, `text {text}`, `divider`, `callout {tone: info | warning, text}`,
  `field {field}`, `table {columns, rows}`, `chart {chartType: bar | line, labels, values}`.
- **Field types:** `text`, `multiline`, `number`, `date` (`YYYY-MM-DD`), `time` (`HH:MM`),
  `select {options: [{value, label}]}`, `toggle`. A field may carry a starting `value`.
- **Actions:** `{id, label, kind}` with `kind` `primary`, `secondary` or `destructive`. A form that
  declares none gets one `Submit`. A block with no actions is display-only and posts no wait.

**Limits:** 40 body components, 20 fields, 8 table columns, 50 rows, 12 chart points, 4 actions,
20 choices per select, 200 characters of title and 4,000 of text.

**Strictness:** an unknown component, prop, field type, action kind or a prop outside the catalogue is
refused, never dropped. Every string must be plain text: a tag (`<b>`, `</`, `<!`), `javascript:` or a
control character is refused. Ids are lowercase letters, digits, `_` and `-`.

`POST /api/chatCall` with `{"botId": "…", "name": "show_block", "arguments": {"block": {…}}}` (what
the bot's `chat` MCP server sends) answers an invalid document with the first broken prop and the
allowed shape, so the agent can fix it in one step.

### Delivery and the answer path

A valid block becomes an **approval entry** (`kind: "permission"`), the same entry every client
already routes to its approval card. Its data carries the usual `title` and `options` (so an older
client still offers the buttons) plus the whole block under `block`:

```json
{
  "title": "Book the trip",
  "toolKind": "block",
  "options": [{"optionId": "approve", "name": "Approve", "kind": "primary"}],
  "status": "pending",
  "block": { "version": 1, "…": "…" },
  "values": null
}
```

The bot's tool waits (up to 10 minutes) on the actor's card, exactly like a Codync approval. The
client answers with the existing method, adding the form's fields:

```json
{"entryId": "…", "optionId": "approve", "values": {"nights": 2, "room": "double"}}
```

`respondPermission` re-checks `values` against the block's own schema (unknown fields, wrong types,
missing required fields and values outside a select's choices are refused), the actor stores them on
the entry and hands `{optionId, values}` to the waiting tool. A cancel (`optionId: null`) closes the
card and the tool reports `{"answered": false, "cancelled": true}`.

### What each client renders

- **Desktop** (`apps/desktop/src/renderer/views/thread/DeclarativeCard.tsx`): Codync's `Stack`,
  `Text`, `Button`, `ChoicePicker`, `Switch`, `Hairline` and its table and SVG chart styles. Fields
  keep what the person typed; **Submit** stays disabled until the required ones are filled.
- **iPhone** (`apps/ios/Kit/Sources/CodyncUI/Thread/DeclarativeCard.swift`): a `CardSection` of rows,
  `Field` + `TextField(...).fieldBox()`, `ChoicePicker`, `Toggle(.codync)`, a `Grid` table and a
  `Canvas`/bar chart, all native.
- **Android: phase 2.** No renderer ships. The card falls back to its title and action buttons, so a
  form is answered on the computer or iPhone. A half renderer (fields that can't report values) was
  deliberately left out; when it lands it follows the same schema and the same `values` answer. The
  terminal UI shows the buttons the same way, without fields.

## K4: shared view state and named actions

### State documents

One document per view, in kv under `viewstate.v1.<view>`:

```json
{"rev": 3, "state": {"open": true, "mode": "canvas", "selected": {"kind": "node", "key": "routine:r1"}}, "updatedAt": 0, "by": "client"}
```

Views are `chat:<botId>` and `routine:<botId>` (a group has no routine canvas). The state is a JSON
object of at most 16 KiB; its shape is the view's own and stays small.

**Deltas** (`ops`) are JSON-patch-like, on object paths only:

```json
[
  {"op": "set", "path": "/thread", "value": "root-1"},
  {"op": "set", "path": "/selected", "value": {"kind": "node", "key": "routine:r1"}},
  {"op": "remove", "path": "/thread"},
  {"op": "append", "path": "/recent", "value": "r2"}
]
```

At most 32 per patch, four keys deep, keys of letters, digits, `_` and `-`. `set` creates missing
parents; `append` only appends to an array.

### Host API

| Method | Body | Answer |
| --- | --- | --- |
| `readViewState` | `{view}` | `{view, rev, state, actions}` |
| `patchViewState` | `{view, baseRev?, ops}` | `{view, rev, state}`, or `{conflict: true, rev, state}` when the caller's copy was stale (nothing is saved) |

A patch saves the next `rev` and broadcasts:

```json
{"type": "viewState", "view": "chat:b1", "rev": 4, "state": {…}, "by": "client"}
```

### Named actions

Each view declares its actions; the host refuses anything else, so a view can't be made to do
something it never offered.

| View | Action | Arguments | Effect |
| --- | --- | --- | --- |
| `chat:<bot>` | `open_thread` | `{rootId}` | Opens that side thread on screen. |
| `routine:<bot>` | `create_routine` | `{name, instruction, schedule}` or `{name, instruction, triggers}` | Creates the routine through `Routines::save` — the API behind `saveRoutine` — and focuses it on the canvas. |
| `routine:<bot>` | `select_routine` | `{routineId}` | Focuses a routine already on the canvas. |
| `routine:<bot>` | `focus_lane` | `{routineId, botId}` | Focuses one agent's lane of a shared routine card (card K14). |

An action saves the view's next state and broadcasts:

```json
{"type": "viewAction", "view": "routine:b1", "action": "create_routine", "arguments": {"name": "Monday review", "routineId": "…"}, "rev": 5}
```

A schedule is the routine editor's own draft (`ScheduleDraft`), so "put a routine for Monday at 8"
becomes `{"kind": "cron", "calendarStyle": "weekly", "weekday": 1, "hour": 8, "minute": 0, "zone":
"Europe/Madrid"}`. The model never writes the database: it asks for an action, and the host's routine
API creates and validates it.

### The agent's tool

`chat` MCP server, tool `view`:

```json
{"op": "list"}
{"op": "read", "view": "routine:b1"}
{"op": "act", "view": "routine:b1", "action": "create_routine", "arguments": {…}}
```

`list` shows the bot's chat and routine canvas with their state and declared actions; `read` shows
one; `act` invokes one. A bot only reaches its own views (`message_bot` reaches another bot's). An
unknown view, kind or action is refused with what the view declares.

### Client wiring

- **Desktop**: `BotSync.reportView(view, ops)` patches with the last rev it saw and swallows failures
  (an older host has no views). `ThreadView` reports `chat:<botId>` (`open`, `thread`) and handles
  `open_thread`; `RoutineFlowView` reports `routine:<botId>` (`open`, `mode`, `selected`, including
  a lane) and follows `create_routine`/`select_routine`/`focus_lane` by reloading the canvas and
  selecting the node or lane.
- **iPhone**: `ThreadView` reports `chat:<botId>` and opens the thread an `open_thread` action names.
- **Android: phase 2**, like block rendering. It neither reports view state nor handles actions yet;
  the host treats a view with no recent delta as empty.

## Tests

- `host/src/declarative/schema_tests.rs`: accept (a full form, defaults, a display-only block) and
  refuse (unknown components/props/types, markup, control characters, every size limit, mismatched
  table/chart shapes, answers checked against the form).
- `host/src/declarative/card.rs`: the card an approval entry carries (options, status, block), the
  refusal with the allowed shape, the answer round-trip (values checked against the form, the tool's
  result), and the display-only case.
- `host/src/views/state_tests.rs`: paths, `set`/`remove`/`append`, and the refusals.
- `host/src/views/tests.rs`: a delta applying and versioning (including a stale `baseRev` conflict),
  unknown views/actions refused, and `create_routine` landing in the routine API.
- `apps/desktop/src/shared/declarative.test.ts`: the form model the card renders (initial values,
  answers sent, completeness) and the chart geometry.

## Limits and follow-ups

- A block is posted in the lane of the actor that asked. A group's room turn or a routine's side
  actor posts to the bot's own chat, the same as `publish_artifact`.
- Only the bot's chat and its routine canvas are views today; a named view is one more `Kind` in
  `host/src/views/state.rs` plus the actions it declares, and each client that shows it.
- The form waits 10 minutes; after that the tool reports a timeout and the card closes.
- Android gets K3 and K4 in phase 2 (renderer and view wiring), documented above rather than shipped
  half done.
