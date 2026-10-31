# Topic 10: messaging channels

## Phase 1: Slack contract, Socket Mode, fake Slack (this commit)

- `host/src/channels/mod.rs` (new): the contract a channel hands the host, `Incoming {channel, person, thread, text,
  files}`. Slack's `accept` (shared by the HTTP route and Socket Mode) builds it with `normalize`; behavior is unchanged
  (the prompt gains the names of attached files). Replies and approval cards join the contract in phase 2, when Telegram
  needs them; adding them now would be dead code.
- `host/src/slack/socket.rs` (new): Socket Mode. `apps.connections.open` with the `xapp-` token (egress policy `Only`
  slack.com) gives a `wss` address that must be on `*.slack.com`; envelopes are acknowledged only after the event was
  queued (a failure is redelivered by Slack, and the inbox drops duplicates by event id); reconnect with a doubling wait
  (1 s to 60 s) that resets on `hello`; reconnects on `disconnect` and when the configuration changed.
- `slack/mod.rs`: `appToken` is optional; the signing secret may be empty only with an app token, and then the HTTP route
  refuses everything (an empty key would verify any body). `slackStatus` reports `socketMode`.
- Tests (`slack/socket_tests.rs`, fake Slack server): queued once despite retry; unknown person acknowledged and
  dropped; reconnect after `disconnect`; refused token; frame handling; socket address check; signed HTTP events,
  replay (old timestamp), wrong signature, duplicate; closed HTTP route in Socket Mode.
- `docs/guides/slack-manual-checklist.md`: what to check with a real workspace.

Not done in phase 1: replies still go out with `chat.postMessage` through `reqwest` directly (unchanged); iOS skipped by
instruction; nothing ran against real Slack.

## Phase 2: Telegram (this commit)

- `host/src/telegram/` (new): `client.rs` (Bot API through `egress`, `Policy::Only(api.telegram.org)`; errors never carry
  the URL because the token is in it; `sendDocument` as a hand-built multipart body), `pairing.rs` (8-character codes from
  a 32-letter alphabet, 10 minutes, one use, void after 5 wrong tries, kept in the key-value store), `commands.rs`
  (`/estado` `/rutinas` `/detener` and English aliases, answered by the host with no model turn), `worker.rs` (one poller
  and one streamer per Telegram bot, reconciled with the configuration every 2 s), `mod.rs` (config in the vault slot
  `telegram.channels`; methods `telegramStatus`, `addTelegramBot` (checks the token with `getMe`), `removeTelegramBot`,
  `pairTelegram`, `setTelegramAllowed`; all but the status are local-only).
- Several named Telegram bots, each assigned to one host bot. Only private chats; a stranger gets no answer unless they
  send a live pairing code.
- A message enters through the `send` API (nonce `tg-<name>-<update id>`), so it is in the bot's chat like any message.
  The streamer writes the agent's text into one Telegram message with `editMessageText` (at most one edit per second)
  and sends the whole reply as `reply.txt` when it is over 4096 characters.
- Approvals: a pending permission card becomes a message with inline buttons whose callback data is
  `p|<entry>|<option>|<fingerprint of the card>`; a tap calls `respondPermission`, so the first answer from any client
  wins and a later one gets the "already answered" toast; the buttons are replaced by the choice once the card settles.
- Tests: pairing (once, expiry, wrong tries), commands (parse, no turn spent), the allowed list, a reply through a real
  scripted agent, a long reply as a file, and an approval answered from buttons (stranger, changed card, first answer
  wins). The fake Bot API checks that every call carries the token.
- Decision: replies go to the bot's main chat, not to a side thread as Slack does; Slack's threads are woven into the
  bot's session code and Telegram has no threads in a private chat.
- Not done: receiving files or photos from Telegram (ignored), webhooks (polling only), nothing ran against real
  Telegram (`docs/guides/telegram-manual-checklist.md`).
