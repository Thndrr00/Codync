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
| Add skills from a GitHub link | A bot installs a skill from a public `github.com` link (the folder that holds `SKILL.md`, the file, or a repository whose top holds one) with its `install_skill` tool, which saves the folder with the link as its source and turns it on for that bot | `host/src/market/github_skill.rs`, `market::install_from_link` in `host/src/market/skills.rs`, `installSkill {url}`, `host/src/chat/skills.rs` |
| Product analytics for all of it | `message_pinned` (`pinned`) for a pin and an unpin, and `setting_changed` with `whatsapp_working_message` for the switch; the media of 0.70.0 sends `media_made` (`kind`, `agent`) | `host/src/analytics/observe.rs`, `host/src/agent/bot/files.rs`, [analytics events](../reference/analytics-events.md) |
| A routine, or an agent in your own chat, reads a connected Slack, Discord, Telegram or WhatsApp chat | `read_channel_messages {source?, limit?}` answers the channel messages this computer delivered to the bot, newest first: text, who and when; it reads the bot's own chat and sends nothing | `host/src/chat/channel_history.rs`, `host/src/mcp.rs`, `api/bots.rs` (`chatCall`) |

The pins surface, its keys and its reasonings are documented in
[chat messages](../features/chat-messages.md#pins) and in the user manual page
`web/content/docs/conversations.mdx` ("Pinned messages").

On Android the release's roster sections of 0.70.0 also carry over unchanged: a pin is a marker on a
message, not a section, so `withMoves` keeps returning its band.

## Not ported, and why

- **Teach a task** (record the pages and clicks of the agent's browser tab, then save it as a skill):
  Codync already turns a walk-through into a skill: `recorded_steps` (`host/src/chat/teach.rs`) reads
  back the conversation's words and tool steps for the bot to save. What 0.69.0 adds is the *browser
  recording* — which pages a person opened and which controls they clicked. Codync does own a CDP
  browser (`host/src/screen/protected_browser.rs`, behind `host/src/browser`), so OpenBot's recorder
  (`src/backend/browser-recorder.ts`) has a tab to watch; the composer entry that starts it and the
  step file it writes are their own change.
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
- **Reading a whole channel's history from a routine, beyond what reached the bot**: OpenBot reads the
  server side, every chat it hosts; Codync's `read_channel_messages` (above) reads what each channel
  delivered to the bot, which is the part a routine can summarize here. A wider reader needs each
  channel's own server-side history, its own change per channel.
- **The message input's pill/card corners, the reworked cards above it and the one-movement
  open/close**: Codync's composer and its queue, waiting block and notice cards have their own shape
  already (see [chat messages](../features/chat-messages.md#motion-and-scrolling)); matching OpenBot's
  row heights would redo motion that is not broken here.
- **Pins of members on import and the notification levels that move with it**: joined-server transfer
  again.

## Checks

The baseline checks and the end-to-end run are shared with the 0.70.0 work of this branch, since both
ports landed together: [`docs/plans/openbot-0.70.0.md`](openbot-0.70.0.md#checks) records the host,
desktop, Android and web checks and the test-side fixes the Windows run needed. This release's own
coverage on top of that: `push`, `steer` and `whatsapp::settings` in the host, and the team tests the
steering default changed, all 29 of them.

`e2e/pins.spec.ts` and the three `e2e/queue.spec.ts` tests pass against the worktree sandbox, covering
the pin list and the sending choice. A linked WhatsApp number is not part of the run (the sandbox has
no phone), so the sheet's own switch cannot be drawn there; `e2e/whatsapp.spec.ts` covers the host
path the switch calls instead — it sets the working message off and on and reads each value back. iOS
could not be built here (no Xcode on this machine), so the iPhone sources of pins and the WhatsApp
switch were compiled by review only.
