# OpenBot 0.69.0 on Codync: what was taken

Reviewed 2026-11-13 against Codync `2.45.1`, from OpenBot nightly tag `v0.69.0`
(`C:/Users/Edwin Villeda/Documents/openbot-nightly`). Same rule as [0.70.0](openbot-0.70.0.md): the
license is ignored on purpose, behavior and text are copied, and the file that holds a piece names
where it came from.

The owner asked for the release's new work after the 0.70.0 port. Most of 0.69.0 is the joined-server
product OpenBot has and Codync doesn't (server transfer and export, server-side per-agent MCP access,
pools of provider accounts, Slack/Discord/Telegram chat reading with that server in the middle). What
maps to Codync's own shape was taken; the rest is listed below with the reason.

## Landed

| 0.69.0 item | Codync shape | Files |
| --- | --- | --- |
| Pins of a message, in a chat and its threads, by people and by the agent | `data.pinned: {by, at}`, `pinMessage`/`pinnedMessages`, the `pin_message` tool, 50 pins per chat | `host/src/chat/pins.rs`, `host/src/store/pins.rs`, `apps/ios/Kit/Sources/CodyncUI/Thread/PinsSheet.swift`, `apps/desktop/src/renderer/views/thread/PinsSheet.tsx`, `apps/android/app/src/main/kotlin/com/codync/android/PinsSheet.kt`, the terminal's `B`/`p` |
| A chat's pins in the chat header, opening the message they name | The pin button lists the chat's pins newest first; a pin opens its thread at the message, with a way back to the latest one | Same three clients, `host/src/chat/pins.rs` (`threadTitle`) |
| A Grok agent makes pictures and clips in the chat | Taken with 0.70.0's media work: the `/image` and `/video` commands, the Grok tool names and their named inputs | `host/src/media/command.rs`, `host/src/media/flow.rs` |
| Electron 45.3.0 (Chromium 156) | `apps/desktop/package.json`; some sites refused the older browser, and the media viewers want the newer one | `apps/desktop/package.json`, `apps/desktop/package-lock.json` |
| A WhatsApp number can answer with only the answer | Every request gets a 👀 reaction; the "Trabajando en ello…" line is a per-number switch, on by default | `host/src/whatsapp/settings.rs`, `incoming.rs` (`acknowledge`), `transport.rs`, the bridge's `react`, the switch in the WhatsApp sheet of the desktop, iPhone and Android apps (the terminal has no WhatsApp sheet) |

The pins surface, its keys and its reasonings are documented in
[chat messages](../features/chat-messages.md#pins) and in the user manual page
`web/content/docs/conversations.mdx` ("Pinned messages").

On Android the release's roster sections of 0.70.0 also carry over unchanged: a pin is a marker on a
message, not a section, so `withMoves` keeps returning its band.

## Not ported, and why

- **Teach a task** (record the pages and clicks of the agent's browser tab, then save it as a skill):
  Codync's agents drive the person's own browser and desktop, not an in-app tab it can record with
  CDP. The recording layer needs a browser Codync owns; it waits for one.
- **Mute agent from the sidebar**: Codync's bots carry their own notification switch (taken with 0.70.0's
  mute row), so the release's per-user level table has nothing left to add here.
- **Pool provider accounts** and **disconnect the first Gemini account**: Codync tracks one account per
  provider runtime (the CLI's own sign-in) and has no account list to pool or to remove an entry from.
- **Library folders, the three Library views, an agent putting a file into a folder, the "Deleted
  agents" group, the Library on the web client**: Codync's files hang on messages instead of a Library
  page, so there is no place for a folder tree yet.
- **Server settings › Transfer** (copy to another server; export a full copy with its switches; the
  web client's transfer): all of it is joined-server work.
- **Per-agent and per-role MCP server access**: Codync connects MCP servers to one bot at a time from
  that bot's settings, so the access list is the connection itself.
- **Signal and relay machines in San Jose and Sydney**: Codync's cloud posts its own placement, nothing
  to copy in the apps.
- **Reading a Slack, Discord, Telegram or WhatsApp chat from a routine**: Codync's WhatsApp, Slack and
  Discord channels are inbound message readers with per-thread answers; a read-back tool for their
  history is its own change, not a copy of OpenBot's server-side reader.
- **The message input's pill/card corners, the reworked cards above it and the one-movement
  open/close**: Codync's composer and its queue, waiting block and notice cards have their own shape
  already (see [chat messages](../features/chat-messages.md#motion-and-scrolling)); matching OpenBot's
  row heights would redo motion that is not broken here.
- **Pins of members on import and the notification levels that move with it**: joined-server transfer
  again.

## Checks

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` on the host:
1531 passed, 2 failed on this Windows machine only (a symlink privilege, `ERROR_PRIVILEGE_NOT_HELD`
1314, and an unpacked-folder cleanup), both unrelated to this work. Desktop `npm run typecheck` is
clean; `npm test` has 812 of 840 passing, its 4 failures being tests that assert English copy on a
Spanish-locale machine. Android `:core:test`, `:app:testDebugUnitTest`, `:app:assembleDebug` and
`:app:lintDebug` are green. iOS could not be built here (no Xcode on this machine), so the iPhone
sources of pins and media were only compiled by review.

The later mute, steer and WhatsApp switch work of 0.70.0/0.69.0 was written after those checks and is
review-only so far: the host's new unit tests (`push::alert_allowed`, `steer`), the catalogs and the
client wiring have not been compiled or run on any machine. The iPhone and Android sheets and the
desktop settings row still need a real device or simulator pass before this ships.
