# OpenBot 0.70.0 on Codync: what was taken

Reviewed 2026-11-13 against Codync `2.45.1`, from OpenBot nightly tag `v0.70.0`
(`C:/Users/Edwin Villeda/Documents/openbot-nightly`, commit `2c1ba1f5a`). OpenBot's license is
ignored on purpose: behavior, text and small pieces are copied, with a one-line origin comment in the
file that holds them.

The owner asked for everything new in that release, naming image generation with GPT Image and Grok
Image, and "multiplayer mode". Media is the whole of the release that Codync lacked; the rest is
either server-side (Codync has no joined servers) or already here.

## Landed

| 0.70.0 item | Codync shape | Files |
| --- | --- | --- |
| Image and Video workflow steps, made with the agent's own subscription tool | Chat commands `/image` and `/video` (`host/src/media/`) and picture/clip steps of a routine flow (`host/src/store/flow_media.rs`, `host/src/routines/flow/`). Analytics: `media_made` (`kind`, `agent`) per made file | Host media module, flow store and API, desktop and Android canvas |
| Transparent background, in each provider's own words | `--transparent` for a picture; Codex asks for clear pixels, Grok gets a flat `#00FF00`/`#FF00FF` | `host/src/media/command.rs` |
| Model menu shows only signed-in providers and the subscription that pays | `mediaModels` lists only the bot's own backend and names its subscription; a group is refused | `host/src/api/media.rs`, pickers in all clients |
| Aspect, length and resolution on the step | Same three on the command and on a media step; each model offers only what it makes | `command.rs`, `MediaSheet` (desktop, Android, iPhone), `MediaPanel` (desktop, Android) |
| Image step used as a reference; pictures into frames and references | `--ref`, `--first`, `--last`; a media step's file is handed on like a bot reply | `command.rs`, `flow::engine` |
| A picture or clip can be a link end | `media:<id>` node keys are interchangeable with bots: same rules, layout and edges | `lib/routine-flow.ts`, `core/RoutineFlow.kt`, both tests |
| Prepare a step from its panel (add a picture/clip step) | **Add a picture step** / **Add a clip step** in the routine panel | `FlowInspector.tsx`, `RoutineFlowDetail.kt` |
| Mute an agent from its menu | The bot's notification switch: `notify: false` mutes it, `null` gives it the computer's choice again; a hidden bot still reaches the phone when it needs you. Analytics: `bot_updated` lists `notify` among its `fields` | `notify` store and API, the mute item in the desktop, iPhone and Android menus, the terminal's **Notify** field, `host/src/remote/push.rs` (`alert_allowed`) |
| A message to a working agent steers the running turn by default | The computer's **Sending** setting (on unless turned off): a user message to a working agent calls `steer`; groups, routines and channel work keep their own turn | `host/src/chat/steer.rs`, `host/src/api/bots.rs`, the **Sending** setting of the desktop and the phones' computer settings, the terminal's palette action |
| Multiplayer mode: who else is in a chat, and who wrote a message | The chat's people are the devices paired with this computer: `presence` answers who else is there and who is typing (present 30 s, typing 4 s), a device's message carries `data.person`, and every client shows the faces and "Ana is typing…" above the message box and the author above a run of messages. Analytics: `presence_shown` (`others`, `typing`) once per chat per host run | `host/src/chat/people.rs`, `host/src/api/bots.rs`, `PresenceRow` and the user row of each client, [chat messages](../features/chat-messages.md#several-people) |
| The chat header shows the agent's title after its name | The agent's own purpose (Codync's `description`) follows the name in the chat header, shortens, and then hides as the header narrows, so the name keeps its room | `ThreadView.tsx` and `thread.css` (desktop), `ThreadView.swift` (iPhone), `ChatScreen.kt` (Android), `host/src/tui/view/chat.rs` |

The picker is in every client's composer (desktop, iPhone, Android); the terminal fills `/image ` or
`/video ` with a new `SlashAction::Media` and the host's refusal note tells the person to describe
what to make. The flow canvas with media steps exists on desktop and Android only, because the iPhone
app has no routine-flow canvas at all (it keeps the routine list); that gap predates this change and
is recorded in [routine flows](../features/routine-flows.md#screens).

The mute item only sets the bot's own switch; Codync has no per-user notification levels. The terminal
has no settings sheet, so its **Notify** field in bot settings is where it is muted, and the palette
carries the steer toggle. Steering is documented in [turns](../features/context-and-memory.md#turns),
mute and the hidden-bot exception in [notifications](../design/push-and-live-activity.md) and
`web/content/docs/conversations.mdx` ("Notifications").

Feature behavior is documented in [media](../features/media.md) and in the user manual page
`web/content/docs/images-and-video.mdx`.

## Not ported, and why

- **Sunra** (Nano Banana Pro, GPT Image 2, Seedream 5.0 Pro, FLUX 3, Seedance 2.0, Kling 4.0,
  Veo 3.1, credits and the credits rail): needs the Apps marketplace sign-in and a balance. It waits
  for a Sunra connector on Codync; the picker already frames models per backend, so it slots in there.
- **Asset steps and pictures dropped from the computer or the Library on a step**: Codync has no
  workflow assets. `--ref`/`--first`/`--last` take paths, and the pickers do not offer an upload yet.
- **The video card of a step with its own play, position, mute and full screen**: a shipped clip plays in
  place in the chat (`AttachmentVideo.swift`, the desktop and Android players) with the system controls,
  not OpenBot's own card in the step.
- **Multiplayer mode's shared channels and name entry**: OpenBot's people join a team server and pick a name
  there; Codync reuses the pairing that already makes a device a person, so a device's own name is the
  author and there is no second sign-in. What is left out is its per-keystroke cursors (Codync sends a
  whole message at a time) and its channel-wide member list (the row only covers the open chat).
- **Server settings** (self-hosted restart, Vault per-agent access, joined-server agent defaults,
  thread limits, dock setting, web sign-in language): Codync has no joined server and Vault, and
  these were outside the release's user-facing media work.
- **TryPost and Adobe Analytics connectors, the payment page, stall profiles and the release's many
  fixes**: cloud/connector work, tracked with their own features, not copied here.

### Two fixes that need nothing here

- **Copy details on a turn that could not continue**: OpenBot's message hid the cause in a log. Codync's
  failure notice carries the reason itself (`agent/bot/updates.rs` writes it), so there is no second
  text to copy.
- **Keep Running / Quit / Cancel when the window closes while agents work**: closing Codync's window
  hides it to the tray and the host keeps running as an installed service (`quit-lifecycle.ts`,
  `host-controller.ts`), so quitting the window never stops an agent. The app already asks before
  quitting when a draft is unsent.

## Checks

Host: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` — 1 535 passed, 2
failed on this Windows machine only (a symlink privilege, `ERROR_PRIVILEGE_NOT_HELD` 1314, and an
unpacked-folder cleanup), both unrelated to this work. The new host tests were run on their own too
(`push::alert_allowed`, `steer`, `whatsapp::settings`).

Desktop: `npm run typecheck` clean; `npm test` 812 of 840 passing, its 4 failures being tests that
assert English copy on a Spanish-locale machine. Android: `:core:test`, `:app:testDebugUnitTest`,
`:app:assembleDebug` and `:app:lintDebug` green. Web: `npm run check:docs` and `npm run lint` clean.

Desktop end to end, against the worktree sandbox and the real app: `e2e/media.spec.ts`, `e2e/mute.spec.ts`,
`e2e/pins.spec.ts` and `e2e/whatsapp.spec.ts` pass together (9.6 s, 12.8 s, 10.1 s, 0.7 s; 4 passed,
35 s), and so do the three `e2e/queue.spec.ts` tests that cover the sending choice. The picker's copy,
its model and aspect choices and the command it writes into the composer are covered; so are the row's
mute item and the notification switch behind it, by what the host stored.

A real generation was run too, once, on the installed Codex CLI (0.166.0, signed in) through a bot of
the sandbox: `/image --model gpt-image --aspect 1:1 a red fox in the snow, watercolour` produced
`red-fox-watercolour.png`, 3 387 080 bytes, attached to the agent's message. The turn took about two
minutes, most of it the agent's own start; `/video` was not run.

Getting that run green on Windows took three fixes, all on the test side. The scripted agent fixtures
were quoted with single quotes, which only quote in `sh`, so on Windows the path split at its first
space and no agent ever started; `fixtureCommand` now quotes per platform. The queue spec asserted row
buttons by their English names, which fail on a Spanish computer, so it pins the window to English like
the pin and media specs do, and its 40-message test has a timeout of its own. The aspect choices are
radios, not buttons. Sandbox memory sat above the host's 85 % refusal threshold, which the run raises
for itself.

The first spec of a run also has the onboarding between the window and the roster, and on this
memory-tight machine it once ran past the 30 s default; the spec files that walk the roster now set a
timeout of 90 s, as the queue one already did.

iOS could not be built here (this machine has no Xcode), so the iPhone sources of media and pins were
only compiled by review.

