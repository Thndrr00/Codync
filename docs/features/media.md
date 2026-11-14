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

- `--model` (default: the backend's first model of that kind), `--aspect` (1:1, 4:3, 16:9, 9:16),
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
(`aspects`, `durations`, `resolutions`, `transparent`, `voices`). A group is refused as "unknown bot":
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
`model`, `prompt`, `aspect`, `duration`, `resolution`, `voice` and `transparent`. Media nodes and bot
nodes are interchangeable link ends — the same canvas rules, layout and edges apply — so a picture
can be handed on to the next step and a bot can hand its reply to a media step. `check_settings`
validates a step against its model before it runs; a run makes the file, shares it in the routine's
thread and hands the file on.

`addRoutineFlowMedia`, `updateRoutineFlowMedia` and `removeRoutineFlowMedia` are the methods; the
canvas payload carries `mediaNodes` next to routines, links and positions, and the `routineFlow`
event still announces a changed canvas.

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

## Not ported

- **Sunra** models (Nano Banana Pro, GPT Image 2, Seedream, FLUX, Seedance, Kling, Veo) need the
  marketplace sign-in and a credits balance; they wait for a Sunra connector.
- **Canvas assets and pictures dropped on a step**, and the video player with its bar in a flow card:
  the clients show the shipped file through the ordinary file view.
- **Multiplayer** is already covered by Codync groups and channels, so it was not rebuilt from
  OpenBot's presence and cursors.
