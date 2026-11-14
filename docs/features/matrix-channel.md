# Matrix channel

The host talks to a Matrix homeserver as a normal account: it logs in with the Matrix Client-Server
API, long-polls `/sync`, accepts the invites it is allowed to accept, answers in the rooms the owner
bound, and uploads files before sending them as `m.room.message` events. Nothing has to be reachable
from the internet, and the access token lives in the host vault.

## Local owner setup

1. Make a Matrix account for the bot on any homeserver (matrix.org, a self-hosted Synapse, Element
   One…). Its user id looks like `@codync-bot:example.org`.
2. Invite that account to each room it should answer in. A room is bound by id (`!opaque:example.org`)
   or by alias (`#team:example.org`); the host resolves an alias before saving it.
3. In **Settings → Channels** on the computer (or the same section on a phone), open **Matrix** and
   paste the homeserver address, the bot's user id, and either an access token or the account's
   password. The password is used once for `POST /login` and never stored; the host keeps the
   returned access token. Leave the token field empty when changing rooms later to reuse the saved
   session.
4. For each room, choose the Codync bot that answers there and list the Matrix user ids allowed to
   write. The host checks every id before saving.

The status screen shows the homeserver, the signed-in user and the bound rooms; it never shows the
token. `matrixStatus` is readable by a paired client, `setMatrixChannel` and `disconnectMatrix` are
the two writes. The desktop and phone apps have the full form; the terminal's Channels sheet shows the
connection and its rooms and can disconnect it (`d`), while credentials are typed in the desktop or
phone app, as with Slack.

## Connecting

- **Access token:** `GET /_matrix/client/v3/account/whoami` confirms the account the token belongs
  to. The token is stored in the host vault under `matrix.channel`.
- **Password:** `POST /_matrix/client/v3/login` with `m.login.password` signs in with a device named
  "Codync". Only the returned `access_token` and `device_id` are kept.
- **Egress:** the homeserver is a URL the person chose, so requests use the host's `UserChosen`
  policy: HTTPS anywhere, plain HTTP only to this computer or its local network. Credentials in the
  URL are refused.
- **Rooms:** aliases go through `GET /directory/room/{alias}`; a room the bot has not joined yet is
  joined when its invite arrives, and a bound room is joined on the first sync.

## Receiving and routing

The long poll uses `GET /sync` with a 30-second timeout, a per-room timeline limit and lazy-loaded
members. The next `next_batch` token is saved in `kv` after the page's events are queued, so a crash
replays the page and the inbox (`matrix_inbox`, primary key `event_id`) drops duplicates. The first
page of a new session is the baseline: its history is not replayed to the bot, but invites on it are
still accepted.

A message is accepted when it is in a bound room, from an allowed user, and either mentions the bot
(`m.mentions.user_ids`, or its user id in the body) or the room already has a conversation with that
bot. Everything else is dropped silently. `m.text`, `m.notice`, `m.emote`, `m.image`, `m.file`,
`m.audio` and `m.video` are read; a media event contributes its file name, as on the other channels.
Edits (`m.replace`) are ignored. Rooms, not threads, are the unit of one conversation, exactly like a
Discord channel.

Accepted messages go through `matrix_inbox` and `matrix_threads` (one root per room) and are claimed
by the bot queue. Replies are written to `matrix_outbox` before the request, split under Matrix's
64 KiB event limit, and sent with `PUT /rooms/{room}/send/m.room.message/{txn}`; the first piece
replies to the message that started the turn. Files are uploaded with
`POST /_matrix/media/v3/upload` (at most 10 files, 25 MiB each) and then sent as one `m.room.message`
per file with `msgtype` chosen from the MIME type. A failure ends in `failed` in
`matrix_outbox`, which the status endpoint lists with the last 50 deliveries.

A message that matches a routine's `message.mention` or `message.created` trigger also starts that
routine with `{source:"matrix", event, room, user, text, eventId, url}`.

## Approvals

Approval cards use the same K5 block model as the other channels, rendered by
`host/src/matrix/render.rs`: the plain-text card is the `body`, the HTML form is `formatted_body`, and
the options are an ordered list. The host also reacts to the card with 1️⃣…5️⃣. A person answers with a
reply that is only a number (`2`) or with the matching reaction; the first answer wins, from any
client, and the card is replaced with its settled text. Reactions and replies are tied to the card's
event id in `matrix_cards`, so a stale card cannot answer another approval.

## Teams

The Adaptive Cards serializer from K5 (`host/src/channels/render.rs`, `teams`) is now used by a Bot
Framework transport: see [teams-channel.md](teams-channel.md). Teams needs a registered Azure Bot
resource and a public HTTPS messaging endpoint, so it is configured separately from a homeserver
account; the registration steps, the JWT check and the connector calls are documented there.

## Limits worth knowing

- Encrypted rooms (`m.room.encrypted`) are not decrypted: bind unencrypted rooms. Matrix's Olm/Megolm
  stack is a separate client-side protocol and is not part of this integration.
- The first sync of a new session is a baseline: messages sent before the host connected are not
  replayed, which avoids answering old conversations after an outage.
- The bot's own messages and edits are ignored. Threads inside a room are not split: the room is one
  conversation, like a Discord channel.
- An access token from a homeserver may be revoked from another device; the host then reports
  `error: …` until the token is saved again.
- The address is used as the homeserver itself: enter the URL the Client-Server API is served from
  (for example `https://matrix.example.org`), not a delegation-only domain. `.well-known` discovery is
  not followed.

## Files

- `host/src/matrix/`: `mod.rs` (config, `matrixStatus`/`setMatrixChannel`/`disconnectMatrix`),
  `client.rs` (C-S API calls and payload builders), `sync.rs` (pagination and invites),
  `ingress.rs` (routing, replies and reactions), `render.rs` (the card), `worker.rs` (outbox and
  approval cards).
- `host/src/store/matrix.sql` and `matrix.rs`: inbox, threads, outbox, card map and deliveries.
