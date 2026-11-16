# The component gallery and its playground

The component gallery is the set of cards Codync itself can draw in a chat, named and versioned by the
host: a metric, a comparison, a timeline, a table, a checklist and a form. A bot asks for one by name
with its props, the host checks the props against a schema it owns, and every client draws that name
with its own controls. A card is never markup.

This follows OpenBot 1.0's gallery (`app/src/components/gallery/` in the reference) as behaviour only:
the same six built-in cards, the same idea of a gallery with live previews and per-bot availability,
and a playground that authors an instance. No OpenBot source is used; OpenBot is PolyForm
Noncommercial, and its components are React.

## How it differs from OpenBot's gallery

| | OpenBot 1.0 | Codync |
| --- | --- | --- |
| What a card is | A React component in the app's own plugin/gallery folder | A catalogue name plus props: data, not code |
| Who renders it | The React component itself | Each client's own controls, from the name |
| Where it can come from | The app's code, and any component its folders carry | Only `components::catalogue`, checked by the host |
| Sandboxing | None: a component runs with the app | No HTML, script, URL or expression can pass validation |
| Publishing | A component in the app | A saved instance (name, props, data requests) with a published/withheld flag per bot |

The security property is the point: the host validates a card, so a card from a bot, from another
computer or from a synced playground draft cannot carry anything a client would execute.

## How it differs from K3 declarative blocks

Both use the same entry (`permission`, `toolKind`) and the same answer path, and the gallery's `form`
card reuses the declarative field types and validator. They answer different questions:

| | K3 declarative block (`declarative-ui.md`) | Gallery card |
| --- | --- | --- |
| Shape | One document with a free `body` of components, plus actions | One named card and its own props |
| Purpose | A form or an approval that needs an answer | A card a person reads: a number, a comparison, a timeline |
| Actions | Up to 4 buttons, each with an id the agent receives | None; only a `form` has a submit, and it returns fields |
| Catalogue | 7 body components, composed | 6 whole cards, each with fixed props |
| Authoring | The bot writes the body | The bot, or the playground, from a form derived from the schema |

Use a block when the person must answer something with buttons; use a gallery card when the bot has
something to show. A card that asks for fields is the exception: a `form` card returns its values by
field id, exactly as a block's form does.

## The catalogue (version 1)

The whole catalogue is `host/src/components/catalogue.rs`. Each component declares its props, the data
functions it may name, and a sample the gallery previews.

| Card | Props (a `*` is required) |
| --- | --- |
| `metric` | `label*`, `value*` (already formatted: `12.4k`), `change`, `trend` (`up`/`down`/`flat`), `caption`, `series` (up to 24 numbers, drawn as a sparkline) |
| `comparison` | `columns*` (2–4), `rows*` (one cell per column), `caption` |
| `timeline` | `events*` (`at`, `title`, `detail`, `tone` = `info`/`warning`/`done`), `caption` |
| `table` | `columns*` (1–6), `rows*` (up to 50), `caption` |
| `checklist` | `items*` (`label`, `done`, `note`), `caption` |
| `form` | `fields*` (1–12: `id`, `label`, `type` = `text`/`multiline`/`number`/`date`/`time`/`select`/`toggle`, `required`, `placeholder`, `options`), `submit`, `caption` |

Bounds: a card's title is 200 characters, a label 120, text 2000, a table cell 200, 4 data requests.
Every string is plain text: an unknown prop is refused rather than dropped, and tags, `javascript:` or
control characters are refused with the field named (`props.rows[2]`).

The schema is a deliberately small subset of JSON Schema, and `listComponents` serves it as
`properties`/`required`/`maxItems` so the playground can build a form from it:

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "columns": { "type": "array", "minItems": 1, "maxItems": 6, "items": { "type": "string" } },
    "rows": { "type": "array", "minItems": 1, "maxItems": 50, "items": { "type": "array" } }
  },
  "required": ["columns", "rows"]
}
```

A host update that changes a prop's shape moves `VERSION`; a saved card that no longer validates is
still listed, marked **Needs updating**, instead of being drawn.

## Data functions (the allow list)

A card may have the host fill props from this computer instead of the bot retyping them. The list is
closed, granted per component, scoped to the bot whose chat the card is in, and bounded:

| Function | Grants | Returns | Bound |
| --- | --- | --- | --- |
| `routines.list` | `table` | columns `Routine`, `When`, `On` | 50 rows |
| `routines.status` | `checklist` | items `{label, done}` from the bot's enabled routines | 20 rows |
| `threads.recent` | `table` | columns `Thread`, `Unread` | 20 rows |
| `bots.roster` | `table` | columns `Bot`, `Kind` for this computer's bots | 50 rows |

A request is `{ "function": "routines.list", "limit": 10 }`. `limit` is clamped to the bound above and
never trusted. A prop the caller also passes wins over the one the host resolves, so a bot can title a
card the computer fills. The resolved values are in the card's props; the card's `source` names the
function. A function is resolved when the card is shown, so a card is a picture of the state then. A
bot with no routines gets an empty table, not a refusal.

## Publishing and withholding

- **Publishing** is per saved instance (`component_instances.published`). A draft is the playground's
  alone; a published card is in the gallery for everyone.
- **Withholding** is per bot, per component (`component_access`). No row means the bot may use the
  component. A row with `published = 0` keeps the component out of that bot's gallery and makes
  `show_component` refuse it for that bot with a reason the agent can act on.
- The catalogue itself is not per bot: withholding applies to the six built-in names, not to their
  props, so two bots never see different versions of the same card.

## The surfaces

- **Desktop:** the **Components** icon in the window's rail (also in the search palette) opens the
  gallery: the six cards with a live preview, the props and data functions each takes, a switch per bot
  to withhold it, and the saved cards with publish, unpublish and delete. The **Playground** tab picks a
  component, edits its props in a form derived from the schema, checks the draft with the host
  (`previewComponent`, data functions resolved), previews it, and saves it as a draft or in the gallery.
- **iPhone and Android:** a **Components** sheet from the computer's settings shows the catalogue with a
  preview and each bot's withhold switch. Authoring props is the desktop's: a schema-derived form of
  rows, timelines and form fields needs the desktop's layout and keyboard.
- **Terminal UI:** `Components` in the command palette lists the catalogue with each card's props, data
  functions and the bots it is withheld from. It is read-only.

## The agent tool

`show_component {name, props, title?, data?, bot?}` (the `chat` MCP server) has the host check the card
and post it in the bot's chat as a `permission` entry with `toolKind: "component"`. A card's validation
error names the field and then lists the catalogue, so the agent fixes it in one step. A `form` card
waits up to 10 minutes and returns `{shown, answered, values}`; every other card returns `{shown}`
immediately. `bot` may only name the calling bot: a card belongs to the conversation that asked for it.

Example:

```json
{"name": "table", "title": "This bot's routines",
 "props": {"caption": "Read from the computer"},
 "data": [{"function": "routines.list", "limit": 8}]}
```

## The host API

| Method | What it does | Who |
| --- | --- | --- |
| `listComponents {bot?}` | The catalogue with schema, sample and availability, the saved cards, and each bot's withhold list | Anyone paired |
| `previewComponent {name, title?, props, data?, bot?}` | Checks and fills one card, as a bot's would be | Anyone paired |
| `saveComponent {id?, name, title?, props, data?, publish?}` | Saves a playground draft | Owner or admin |
| `publishComponent {id, published}` | Shows a saved card in the gallery, or takes it out | Owner or admin |
| `deleteComponent {id}` | Deletes a saved card | Owner or admin |
| `setComponentAccess {bot, component, published}` | Lets a bot use a component, or withholds it | Owner or admin |

Only an owner or an admin authors: the gallery is the computer's, and a member's phone showing a card
every other bot draws would be a policy change.

## Where it lives

| Part | File |
| --- | --- |
| Catalogue, props and samples | `host/src/components/catalogue.rs` |
| Validation and the card document | `host/src/components/schema.rs` |
| Data functions and their bounds | `host/src/components/data.rs` |
| Gallery and playground API | `host/src/components/instances.rs` |
| `show_component`, the card, a form's answers | `host/src/components/card.rs` |
| Storage | `host/src/store/components.rs` |
| Desktop | `apps/desktop/src/shared/components.ts`, `renderer/views/components/`, `renderer/views/thread/ComponentCard.tsx` |
| iPhone | `apps/ios/Kit/Sources/CodyncKit/Models/ComponentCard.swift`, `CodyncUI/Thread/ComponentCard.swift`, `CodyncUI/Components/ComponentsSheet.swift` |
| Android | `apps/android/app/src/main/kotlin/com/codync/android/ComponentCard.kt`, `ComponentsSheet.kt` |
| Terminal UI | `host/src/tui/manage/components.rs` |
| Docs for people | `web/content/docs/artifacts.mdx` |

## Tests

In `host/src/components/tests.rs`:

- a form card round-trips the answers the person gave, and refuses a missing required field, a field the
  card never asked for, and the wrong type;
- a display-only card is shown at once, carries no actions, and is not a declarative block;
- a card names the field that broke the catalogue (a row/column mismatch, an unknown prop, a missing
  required prop, markup, an unknown component);
- withholding keeps a component from that bot and no other, and publishing it again gives it back;
- the gallery lists every component with its schema, sample, data functions and bots;
- a published card is in the gallery and a draft is not; a card the catalogue refuses is never saved;
- a data function fills a card from this computer, and a prop the caller wrote wins;
- a card refuses a data function it was not granted, and a bad `limit`;
- every built-in sample passes its own schema;
- the tool lists the components it can show.

The chat card's transport (an actor posting the entry and a client answering it) is not unit-tested
here: `card::card` and `card::answers` are the two ends of that round trip, and the entry shape is the
one the declarative block tests already cover.
