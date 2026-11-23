# Images and video

A bot makes a picture or a short clip with the tools of its **own agent**, paid by the subscription
that agent is signed in with: Codex's `image_gen`, and Grok's `image_gen`, `image_edit`,
`image_to_video` and `reference_to_video`. No API key is added and no second runner starts, so a bot
already signed in can make files at once. Ported from OpenBot 0.70.0 (workflow media steps); the host
holds all of it (`host/src/media/`, `host/src/api/media.rs`).

## What a request is

A chat message that starts with `/image` or `/video` and carries the settings as flags
(`host/src/media/command.rs`). The picker in the composer writes it, so a person only describes what
to make. The host, not the agent, decides what the flags mean:

```
/image --model gpt-image --aspect 16:9 --transparent a red bicycle on a white wall
/video --model grok-imagine --aspect 9:16 --duration 6 --resolution 720p --voice eve a paper boat in a gutter
```

- `--model` (default: the backend's first model of that kind), `--aspect` (1:1, 4:3, 16:9, 9:16, 4:5, 5:4),
  `--transparent` (pictures only), and for a video `--duration` (6 or 10 seconds), `--resolution`
  (480p, 720p) and `--voice` (ara, eve, leo, rex).
- `--first`, `--last` and `--ref <path>` name a first frame, a last frame and up to 14 reference
  pictures; an image uses the references as an edit.
- An unsupported value falls back to the model's nearest choice rather than failing the turn, as
  OpenBot does; only an unknown model fails.
- A bare `/image` or `/video` is answered with a note that describes what to make; the picker never
  sends one. The command must start the message.

## Models

`host/src/media/catalog.rs` lists what each backend can run. A bot only ever offers the models of
its own backend, and the answer names the subscription that pays.

| Id | Backend | Makes | Transparent | Voice |
|---|---|---|---|---|
| `gpt-image` (GPT Image, OpenAI, paid with ChatGPT) | `codex` | image | yes, clear pixels | no |
| `grok-imagine` (Grok Imagine, xAI, paid with Grok) | `grok` | image, video | yes, one flat colour | yes |

A transparent background is asked for in words the tool obeys: Codex makes clear pixels, Grok's
tools cannot, so they get a flat `#00FF00` (pictures) or `#FF00FF` (video frames) the person can key
out. A backend without a media model (Claude, others) answers a refusal that names it.

## API

`mediaModels {botId}` (`host/src/api/media.rs`) answers what that bot can make: `agent`, `installed`,
and a `models` list with `id`, `name`, `maker`, `subscription`, `kinds` and `capabilities`
(`aspects`, `durations`, `resolutions`, `transparent`, `voices`). Each entry also carries
`available`, `batchSizes` (`[1, 2, 3, 4]`) and `batchCredits`: when the service prices the model,
`credits` is the price of one result and `totals` is that price times each batch size, computed by
the pure `media::batch_credits`. A subscription model has no price, so its totals are `null`; no
price, token or description is written to logs or analytics. A group is refused as "unknown bot":
media is offered to a bot only. The usual `control` scope applies.

## Sharing

The file the tool left is published like a bot's own `send_file`: a snapshot under
`~/.codync/bots/<bot>/files/<fileId>`, a final entry with `files: [{id, name, size, sha256}]` and
`data.media = {kind}`. The prompt is kept as the entry's text. `host/src/agent/bot/updates.rs`
dedupes a share whose file the tool already published (`mediaShared`), so the picture appears once.
A share happens quietly: a failure posts a `Notice` instead of breaking the turn, and
`media_failed` records why. Each made file also counts as `media_made` (`kind`, `agent`), never the
description ([analytics events](../reference/analytics-events.md)).

## Routine flow steps

A routine flow can hold a **media step** (`host/src/store/flow_media.rs`,
`host/src/routines/flow/`): a picture or clip node keyed `media:<id>` with `routineId`, `kind`,
`model`, `prompt`, `aspect`, `batch`, `duration`, `resolution`, `voice` and `transparent`. Media nodes
and bot nodes are interchangeable link ends — the same canvas rules, layout and edges apply — so a
picture can be handed on to the next step and a bot can hand its reply to a media step. `check_settings`
validates a step against its model before it runs; a run makes the file, shares it in the routine's
thread and hands the file on.

`batch` is 1 to 4 (default 1; an out-of-range value from a client is bounded by the pure
`media::media_batch`, and the store refuses it). A batch of N makes N results through the same
provider call path, one call at a time, checking the flow's cancellation before each and while one
is running. Every made file is handed on, at most four files are stored on the step, and each result
gets its own `pending`, `succeeded` or `failed` entry in the step's saved settings with the error
text a client shows beside **Retry**. The statuses are re-saved and broadcast as the batch runs, so
a client can show progress under still-pending results.

A step whose model is not in the catalog the service can make now stops with the fixed error
"The model of this step is not available" — first at `begin`, then again at `make` against the live
Sunra list — before any provider call, so no credits are used. An unavailable Sunra model is not
listed at all; the catalog entries a client receives carry `available: true`.

`addRoutineFlowMedia`, `updateRoutineFlowMedia` and `removeRoutineFlowMedia` are the methods
(`updateRoutineFlowMedia` also takes `batch`); the canvas payload carries `mediaNodes` next to
routines, links and positions, and the `routineFlow` event still announces a changed canvas.

## Screens

- Composer (all clients): the slash menu's `/image` and `/video` open a sheet listing the models of
  that bot, the frame, and for a video its length, resolution and voice; **Use** writes the command
  with the description kept from the box. States: host not answering, checking, no image tool (both
  by the agent and by the model), agent not installed, and the model list. The iPhone and Android
  sheets carry **Use** as a `checkmark` icon button and present through `.codyncSheet` and
  `CodyncSheet`.
- Desktop and Android also edit media steps in the routine flow canvas: **Add a picture step** and
  **Add a clip step** in the routine panel, and a `MediaPanel` inspector with the model, the settings,
  the description, save and remove, and what the step may hand its file on to.
- The terminal UI has no picker: `/image` and `/video` fill `/<id> ` and the person types the flags
  or sends the command bare to get the host's note.
- **iPhone has no routine-flow canvas** (the flow is desktop, Android and TUI only), so media steps
  there are read and edited from the other clients; the phone still has the composer picker.

## Compatibility

Additive: `mediaModels`, the commands, the `media:` node keys, the methods and the event fields.
`minApp` and `minHost` do not change, and a client that knows none of it shows the command as plain
text and ignores `mediaNodes`.

## Files a WhatsApp number received

Ported from OpenBot 0.72.0: a linked number keeps the files it received along with the text, so a
routine can read what came with a message that never reached the bot. `save_media`
(`host/src/whatsapp.rs`) writes the file when the message arrives, before the mention and allow
checks, and `whatsapp::history` keeps its path beside the text in a `file` column (relative to the
data folder, added to a table made before the column on first use). Unlinking the number, or a
message leaving the 90-day or 20 000-row retention, deletes the file with its row. A message revoked
or dropped from the input queue before it reaches history also has its saved file deleted, so no
file is left behind without a row. The kept media of one number is also capped at
2 GB: once the files pass that, the oldest are removed first and only their rows'
`file` paths are cleared, so the messages and their text stay.

## Sunra

Sunra models (Nano Banana, Seedream, FLUX, Seedance, Kling, Veo and others) are a second kind of
media model (`host/src/media/sunra.rs`). They are paid with the person's Sunra credits, not with an
agent subscription. The **Sunra** app in the marketplace installs a remote MCP connector at
`https://mcp.sunra.ai`; the person signs in to it in Extensions. With no installed, signed-in
connector, Sunra is simply unavailable: the model list is empty, `sunraCredits` answers `null`, and
a flow that does not use Sunra behaves as before.

The host reads Sunra's live `list_models` into the same catalog shape as the fixed models, with ids
`sunra-<id>` and the model's aspects, durations, resolutions and transparency. It also reads
`get_balance`: a team member's `memberQuota.effectiveQuota` gives the available credits and quota,
and a personal account's `availableBalance` gives the available credits with no quota. The methods
`sunraModels` and `sunraCredits` return `{available, models}` and `{available, credits}`; no token,
address or description is ever returned or logged.

A generation uploads each reference picture through `get_upload_url`, creates the image or video with
`auto_confirm`, polls `get_job`, and downloads the file only from `cdn.sunra.ai`, `cdn.ponpon.ai` or
`assets.sunra.ai` through the egress allowlist. Every hop must be https, every call has a timeout,
and the result is kept as a bot file named `sunra-image.png` (or the matching extension). The model's
live capabilities say how many references it takes and whether it takes frames or a clear
background; a last frame is only sent with a first one. The pictures uploaded for a step are kept
by the service for up to **three days**, not one (`media::REFERENCE_RETENTION_DAYS` and the pure
`reference_expired` decision; the window is also in the catalog JSON for clients to show).

## Not ported

- **Canvas assets and pictures dropped on a step**, and the video player with its bar in a flow card:
  the clients show the shipped file through the ordinary file view.
- **Multiplayer** is already covered by Codync groups and channels, so it was not rebuilt from
  OpenBot's presence and cursors.
