# A2A between your computers

Codync speaks [A2A](https://a2a-protocol.org) (Agent2Agent) in both directions for bots that are
offered to a codync-network. It is implemented from the public protocol with no A2A package:

- each opted-in bot has an **agent card** and a local JSON-RPC endpoint that serves
  `message/send` and `message/stream` (Server-Sent Events);
- a bot can ask another computer's A2A agent a bounded task with the **`a2a_ask`** tool.

The cross-computer path does not open an A2A port. The JSON-RPC request travels as a
codync-network letter, so the receiving computer's roster identity, rules, budgets and the bot's own
approval cards apply exactly as they do for `ask_bot`.

## Opting in

A bot is an A2A agent under the same per-bot switch that offers it to the network: **Offered to the
network** in the bot's settings, on by default. A hidden bot, a group and a bot kept off the network
have no card and refuse A2A requests. The switch applies to every network the computer is in, like
the rest of the network offering.

## The card

```http
GET /a2a/{bot}/.well-known/agent-card.json
Authorization: Bearer <this computer's local API token>
```

`{bot}` is the bot's id or its name (case-insensitive). The card is built from the bot's name and
description:

```json
{
  "name": "Reviewer",
  "description": "Reviews diffs.",
  "url": "http://127.0.0.1:19222/a2a/<bot id>",
  "version": "2.49.0",
  "protocolVersion": "0.3.0",
  "preferredTransport": "JSONRPC",
  "capabilities": {"streaming": true, "pushNotifications": false},
  "defaultInputModes": ["text/plain"],
  "defaultOutputModes": ["text/plain"],
  "skills": [{"id": "reviewer", "name": "Reviewer", "description": "Reviews diffs.", "tags": ["codync"]}],
  "metadata": {"computer": "desk", "agent": "reviewer"}
}
```

For another computer, `a2aAgents` composes the same shape from what that computer offers the network
(`codync://reviewer@desk`, `preferredTransport: "codync-network"`). The authoritative card stays on
the computer that runs the bot.

## The local endpoint

`POST /a2a/{bot}` with a JSON-RPC body, authenticated exactly like AG-UI: loopback plus the host's
bearer token, subject to the browser-origin and rebinding guards.

```json
{"jsonrpc":"2.0","id":"1","method":"message/send","params":{"message":{
  "role":"user","messageId":"m1","contextId":"c1","parts":[{"kind":"text","text":"review this"}]
}}}
```

- `message/send` runs one turn of the bot in its own chat and waits for the `turnResult`. The result
  is a completed task: `{kind:"task", id, contextId, status:{state:"completed"}, artifacts:[{...,
  parts:[{kind:"text", text}]}]}`. A failure is a JSON-RPC error (`-32000`).
- `message/stream` answers `text/event-stream`. Each event is `data: <json>` and a blank line, with
  a `status-update` (`working`), one or more `artifact-update` chunks (`append`) and a final
  `status-update` (`completed`). A client that disconnects stops the turn.
- Unknown methods are a `-32602` JSON-RPC error. Only text parts are read.
- The turn runs through `chat::messages::post_in_lane`, so the bot's approvals appear in its chat as
  usual; an A2A client that needs an approval waits while the person answers in Codync.

This endpoint is for this computer's apps and helpers. It is not exposed to the internet, and it
does not bypass the local API's authentication.

## Between computers

A bot calls the tool:

```json
{"agent": "reviewer@desk", "message": "Review the diff in ~/code/shop.", "timeoutSeconds": 600}
```

1. The host resolves the address with `network::recipient` (the network is named when the computer
   is in several: `reviewer@desk.home`). A local bot is refused with "use ask_bot".
2. The request is a letter whose `meta.a2a` carries the JSON-RPC `message/send` envelope, with the
   ordinary message budget reservation and hop count.
3. The receiving computer checks the roster (`codync_network`), the per-bot offering, the network's
   rules and its budgets before the letter reaches `a2a::intercept`. A refused letter never reaches
   the bot; the sender gets the refusal as its tool error.
4. The bot runs through `chat::team::a2a_ask`: its normal actor, its own folder, tools and approval
   cards, a 10-minute bound, and an "A2A request from reviewer@desk" notice in its chat.
5. Its final text becomes the completed task's artifact, carried back as a notice letter with
   `meta.a2a`. The caller's `a2a_ask` returns the task result. A failure or timeout is a JSON-RPC
   error with the reason.

The same answer rules apply: an answer is allowed back from the computer that was asked, even if the
rules changed while the task ran. The receiving computer's user can stop the bot, which ends the
task as failed. There is no streaming over the network: `message/stream` from another computer
returns one completed task. Local streaming is the SSE endpoint above.

## API and tools

| Method / tool | What it does |
| --- | --- |
| `a2aAgents` | The A2A agents the other computers offer, as cards. |
| `a2aCard {agent}` | One of those cards. |
| `a2aSend {botId, agent, message, timeoutSeconds}` | An app asks on a local bot's behalf. |
| `a2a_ask` | A bot tool: asks another computer's A2A agent and waits for the result. |

`a2a_ask` is in the team tool set with `list_bots`, `message_bot`, `ask_bot`, `delegate_batch` and
`run_pipeline`. It is bounded at 10 minutes, uses the receiving computer's rules and budgets, and
respects the recipient's approvals; it is the A2A equivalent of `ask_bot`, not a way around it.

## Limits

- No push notifications and no A2A file parts: text only. A bot's own files stay in its folder.
- The local SSE endpoint streams; the cross-computer path returns one task.
- A remote card is compiled from the directory; it is not fetched from the other computer.
- A bot kept off the network, a hidden bot and a group are not A2A agents.
- An A2A task is a bot request, not a new user instruction: the recipient is told to work within its
  own permissions and to return the result addressed to the asking agent.

## Files and tests

- `host/src/a2a/`: `mod.rs` (waiters, methods and the tool), `card.rs` (agent cards),
  `protocol.rs` (request parsing, task/artifact/SSE builders), `server.rs` (the local endpoint),
  `client.rs` (letters, interception and refusal).
- `host/src/chat/team.rs`, `blocking.rs` (`a2a_ask` and `Back::A2a`),
  `host/src/chat/team/remote.rs` (interception before the normal request paths), `host/src/hub.rs`
  (the waiter map).
- Tests: the card's fields (`card.rs`), text-part parsing and task/artifact/SSE mapping
  (`protocol.rs`), an unoffered bot being refused (`admit` and `agent_available` in `tests.rs`), and
  the network rule and reply-edge tests in `host/crates/network` and
  `host/tests/network_e2e.rs`.
