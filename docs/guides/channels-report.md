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
