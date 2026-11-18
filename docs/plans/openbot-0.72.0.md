# OpenBot 0.72.0 on Codync: what was taken

Reviewed 2026-11-17 against Codync `2.47.0`, from the nightly repo at `v0.72.0` (the local copy was
updated first: `v0.71.0` re-checked and unchanged, `v0.72.0` was new). Same rule as
[0.70.0](openbot-0.70.0.md): behavior and text are copied, adapted to Codync's own screens, and the
file that holds a piece names where it came from.

0.72.0 is a feature release with a clear split: hosted-product work (storage sizes, Umbrel, the Docker
setup page, server felt plush), joined-server work, marketplace connectors, provider sign-in flows,
and a handful of things Codync can hold today.

## Landed

| 0.72.0 item | Codync shape | Files |
| --- | --- | --- |
| A `WhatsApp` agent can find earlier messages by words or by time | The channel history tool takes `query` (every word, any case) and `since` (Unix ms) beside `source` and `limit`, so a routine or an agent searches what the channels delivered instead of only reading the newest; the scan stops as soon as it passes `since` | `host/src/chat/channel_history.rs` (`words`, `holds`, `read`) |
| Your bubbles take the agent's avatar colour (Solid / Tint / Classic in Settings › Appearance) | **Settings › Appearance › Your bubbles** with three choices: Solid (the default) fills the person's bubbles with the bot's avatar colour, Tint mixes 14% of it in (22% in dark), and Classic keeps the chat's bubble as it stands. A group, a bot with a photo and a channel keep Classic; a Solid fill takes the ink that reads on it (4.5:1). A device preference, applied in each client's user row | `apps/desktop/src/renderer/views/thread/ChatRows.tsx` and `lib/theme.ts`, `apps/android/app/.../ChatRows.kt` and `design/Appearance.kt`, `apps/ios/Kit/Sources/CodyncUI/Thread/ChatRows.swift` and `CodyncKit/Design/Appearance.swift` |

## Already here, verified in the code

| 0.72.0 item | Where Codync already does it |
| --- | --- |
| An OpenCode agent, a custom ACP agent or a Devin agent takes a message into its running turn | Any backend whose ACP advertises `_session/steering` steers (`host/src/agent/bot/steer.rs`), and an agent that does not advertise it keeps the queue; the release raises the same gate. |
| Connectors arrive through the catalog rather than the app | Codync imports its catalog from OpenBot's lists (`tools/import-openbot-catalog.mjs`), so AppSignal, Cloud 66 and HubSpot are a catalog import when they are wanted, not app code. |

## Not ported, and why

- **Hosted storage sizes, Umbrel, the Docker setup page, the server's felt plush**: the hosted product
  and its packaging.
- **Joined-server items** (sign-in links of a server, memory changes pushed to a phone, "only the owner
  can ask for a browser profile change"): joined server, as in [0.69.0](openbot-0.69.0.md).
- **Several accounts per provider and "add account with code"**: Codync tracks one account per runtime;
  multiple accounts are a feature of its own.
- **Find models for a custom provider**: Codync has no "find models" flow; a custom endpoint is typed.
- **Press Enter to send a recorded voice message**: Codync's phones send from their own controls; the
  desktop has no record-then-send box.
- **Pace mark on a plan-limit bar**: Codync's usage bars show the window and its reset; the mark needs
  the window's start and reset from every provider, which Codync does not read yet.
- **`@` mentions finding threads by their messages, avatar tooltips in a narrow sidebar, the Guides
  articles**: product surfaces Codync does not have in that shape; each would be its own change.

## Checks

Host `cargo fmt --check` (through `rustfmt --check`) is clean on the changed file, and the module has
its own test for the query words. The release's own gate is the branch's, recorded in
[0.70.0](openbot-0.70.0.md#checks).
