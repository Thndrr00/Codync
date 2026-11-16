# Signal channel

Signal is a messaging channel the way Matrix is: the host talks to a `signal-cli` daemon the
person runs on the computer, and never links a number itself. The daemon owns the Signal account
(its keys and its linked devices); the host keeps the number in its credential vault and calls the
daemon's local JSON-RPC endpoint. No Signal or Codync server is involved.

## Link a number and start the daemon

`signal-cli` can link a number as a device, or register one it owns. Either is done once, outside
Codync:

```sh
# Link the account as a device: scan the QR code in Signal > Linked devices.
signal-cli -a +15551234567 link

# Or register and verify a number the person owns.
signal-cli -a +15551234567 register
signal-cli -a +15551234567 verify 123456

# Then keep the daemon running while Codync runs.
signal-cli -a +15551234567 daemon --http 127.0.0.1:7583 --receive-mode=on-start
```

`--receive-mode=on-start` makes the daemon start receiving as soon as it starts instead of when a
client asks. The host does not spawn `signal-cli` and does not ship it: a computer without it shows
**signal-cli is not running** and the exact commands, never a pretend connection.

## The contract the host speaks

- `POST /api/v1/rpc` with a JSON-RPC 2.0 body. The host uses:
  - `listAccounts` with `{}`: the linked numbers, as `[{"number": "+15551234567", "uuid": "..."}]`.
    This is also the reachability check.
  - `send` with `{"account": "+15551234567", "recipient": ["+15557654321"], "message": "..."}` for
    a direct chat, or `"groupId": "<id>"` instead of `recipient` for a group. Files go in
    `"attachments": ["/absolute/path"]`. The result's `timestamp` identifies the message and is the
    id a reply quotes.
  - `getAttachment` with `{"account": "+15551234567", "attachmentId": "<id>"}`: base64 `data`,
    which the host writes under `signal-media/<generation>/` and then into the bot's uploads.
- `GET /api/v1/events` as Server-Sent Events. Each `data:` frame is a JSON-RPC notification. The
  host reads `method: "receive"` frames:

```json
{"jsonrpc":"2.0","method":"receive","params":{"account":"+15551234567","envelope":{
  "source":"+15557654321","sourceNumber":"+15557654321","sourceUuid":"...","sourceName":"Ana",
  "timestamp":1731500000000,
  "dataMessage":{
    "timestamp":1731500000000,"message":"deploy please",
    "groupInfo":{"groupId":"<base64>","groupName":"Team"},
    "attachments":[{"contentType":"image/png","fileName":"a.png","id":"...","size":1234}],
    "quote":{"id":1731499990000,"author":"+15557654321","text":"Needs you: Run a command"},
    "mentions":[{"start":0,"length":1,"author":"+15551234567"}]
  }}}}
```

Receipts, typing indicators, sync messages and edits (`editMessage`) are ignored. The stream has no
cursor: a reconnect starts a new stream, and the inbox (`signal_inbox`, primary key
`message_id = dataMessage.timestamp`) drops anything already seen. The host reconnects every few
seconds while the daemon is configured; the status is `ok` or `starting`, and a daemon that is not
there is `notRunning: <why>`.

The daemon address may be `http://` on this computer or its own network and must be `https://`
elsewhere. Credentials in the URL and a path are refused. Requests use the host's `UserChosen`
egress policy, so a daemon on another of the person's computers is reached the same way.

## Settings

**Channels** on the desktop, iPhone, Android and the terminal UI (`Channels` in `^k`) show the
number, the daemon, each bound chat, its state and the last deliveries:

- the Signal number, in E.164 (`+15551234567`);
- the daemon's base URL (default `http://127.0.0.1:7583`);
- one or more chats, each a phone number (a direct chat) or `group:<id>`, the Codync bot that
  answers there, and 1–100 allowed Signal numbers or UUIDs.

The desktop, iPhone and Android have the full form. The terminal UI shows the connection, its
chats and can disconnect it (`s`); credentials and chats are typed in an app, as with Slack. On
save the host checks that the daemon answers and that the number is one it has linked. If the
daemon is not running, the configuration is still saved and the status tells the person what to
start, so a host can be set up before `signal-cli` is installed.

A bound chat also has **Channel memory**, **notes** and a **picture**, like Matrix. The number is
stored in the vault slot `signal.channel`; the account's keys never leave the daemon.

## Receiving and replying

A message is accepted when it is in a bound chat, from an allowed sender, and:

- in a direct chat, always;
- in a group, when the message mentions the bot (a Signal `mentions` entry for its number, or its
  number in the text) or the group already has a conversation with the bot.

Accepted text and attachment names become one bot prompt. Attachments are downloaded through the
daemon, capped at 10 files and 20 MiB in total, and saved to the bot's uploads so its agent can
open them; a failed download still tells the bot the file's name. Groups are one conversation per
group, like a Discord channel.

Replies go to the same chat: text split at Signal's practical message size, with files carried in
the same `send` request. The outbox is `signal_outbox`; a request interrupted mid-send is reported
as `unknown`, not retried automatically. The status lists the last 50 deliveries.

## Approvals

Approval cards use the same K5 block model as the other channels and render as plain text with
numbered options (`channels::render::signal`). Signal has no buttons and cannot edit a sent
message, so the rule is explicit in the card:

> Reply to this message with the number of an option:

A person answers by quoting the card and sending only the number (`2`, `2.` or `2)`). The card's
timestamp is mapped to its permission entry in `signal_cards`, so a stale quote cannot answer a
newer approval. The first answer from any client wins; the settled line is sent as a new message.

## Limits

- `signal-cli` must be installed and linked by the person; Codync cannot assume it.
- A daemon that stops is a state, not an error loop; queued prompts wait, and replies are not
  duplicated.
- Messages from before the host connected are not replayed: the inbox starts at the stream's first
  envelope and deduplicates by message id.
- Edits and reactions are not applied to cards; Signal cannot edit a message the host sent.
- Voice notes are not transcribed on this path; they arrive as files.
- `getAttachment` needs an attachment id from the envelope. If a signal-cli version refuses it without
  the sender's address, the host still tells the bot the file's name and does not fake a download.
- Read receipts and typing indicators are not sent.

## Files

- `host/src/signal/`: `mod.rs` (vault config, `signalStatus`/`setSignalChannel`/`disconnectSignal`
  and per-chat memory/icon), `client.rs` (JSON-RPC and SSE, payload builders), `sync.rs` (event
  stream and reconnects), `ingress.rs` (allow list, groups, mentions, attachments, quoted approval
  answers), `render.rs` (the card), `worker.rs` (outbox and approval cards).
- `host/src/store/signal.rs` and `signal.sql`: inbox, threads, outbox, card map and deliveries.
- Tests: payload builders and SSE frames (`client.rs`), the allow list, chat parsing and approval
  mapping (`ingress.rs`, `tests.rs`), the card text (`render.rs`).
