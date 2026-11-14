# AG-UI

Codync speaks [AG-UI](https://docs.ag-ui.com) in both directions, implemented from the public protocol with no
AG-UI package: the host serves a bot as an AG-UI agent, and a bot can run a remote AG-UI endpoint as its agent.

| Card | Where | What |
| --- | --- | --- |
| K1 | `host/src/agui/server.rs`, `mapping.rs`, `input.rs`, `events.rs` | `POST /agui/{bot}` streams one bot turn as an AG-UI run. |
| K2 | `host/src/agui/shim.rs`, `remote.rs`, `endpoint.rs` | A bot on the `agui` backend answers through a remote AG-UI endpoint. |

## The server: `POST /agui/{bot}`

The route lives on the local API and is authenticated exactly like the rest of it: loopback, the host's bearer
token (`Authorization: Bearer <token>`), the same browser-origin and rebinding guards. `{bot}` is the bot's id or
its name (case-insensitive).

The request is an AG-UI `RunAgentInput`:

```json
{
  "threadId": "…",
  "runId": "…",
  "messages": [{"id": "1", "role": "user", "content": "Hello"}],
  "state": {},
  "tools": []
}
```

Only the newest user message becomes the turn; the bot keeps its own conversation history. A missing `threadId`
or `runId` is generated. The body is limited to 1 MiB, and one text delta or tool payload per event to 8 KiB and
16 KiB, so a single SSE line stays bounded.

The response is `text/event-stream`; each event is one `data: <json>` line and a blank line. The run starts with
`RUN_STARTED`, then a `STATE_SNAPSHOT` of the state the client sent (Codync has no view state of its own yet;
K4 would add `STATE_DELTA` patches), and ends with `RUN_FINISHED` or `RUN_ERROR`. A client that disconnects
before the end stops the bot's turn.

### Event mapping (Codync → AG-UI)

Every transcript entry of the turn (same bot, same turn, the bot's own chat, not a side conversation) is mapped
from the host's event bus. This is a view of the existing bus: there is no second agent loop.

| Codync entry / state | AG-UI events |
| --- | --- |
| run begins | `RUN_STARTED` with `threadId`, `runId` |
| request's `state` | `STATE_SNAPSHOT` with `snapshot` |
| `agent` entry, `final: false` | `TEXT_MESSAGE_START` (`messageId` = entry id, role `assistant`), then `TEXT_MESSAGE_CONTENT` deltas (entry data carries the whole segment so far; the host sends only the unseen suffix) |
| `agent` entry, `final: true` | the last `TEXT_MESSAGE_CONTENT`, then `TEXT_MESSAGE_END` |
| `tool` entry, first seen | `TOOL_CALL_START` (`toolCallId`, `toolCallName` = title), `TOOL_CALL_ARGS`, `TOOL_CALL_END` |
| `tool` entry, `completed` / `failed` | `TOOL_CALL_RESULT` (`messageId`, `toolCallId`, `content`, role `tool`), once |
| `permission` entry (created or answered) | `CUSTOM` name `codync.approval`, `value`: `entryId`, `botId`, `title`, `detail`, `toolKind`, `options`, `selected`, `status`, and `answer: {method: "respondPermission", params: {botId, entryId}}` |
| `notice` entry | `CUSTOM` name `codync.notice` (`entryId`, `text`, `style`) |
| `plan` entry | `CUSTOM` name `codync.plan` (`entryId`, `entries`) |
| user message gets `turnResult` `done` / `paused` / `truncated` / `stopped` | `RUN_FINISHED` with `result` = the final text (or `null`) |
| `turnResult` `failed` / `interrupted` / `waitingForSignIn` | `RUN_ERROR` with a human message and `code` = the state |
| bus lag, result not yet known | `CUSTOM` name `codync.resync`; the host also checks the stored result first |
| 30 minutes without ending | `RUN_ERROR` `timeout`, and the turn is stopped |

An approval is answered through the host's existing method, with the body the custom event documents:

```json
{"botId": "…", "entryId": "…", "optionId": "allow"}
```

`POST /api/respondPermission` with that body resolves the card; the change comes back as another
`codync.approval` custom event with `status: "answered"` and `selected`.

### What the server does not do

- `tools` in the request is accepted and ignored: the bot's tools are its own MCP servers, not the client's.
- `state` is echoed, not changed. There is no view state until K4 lands.
- It streams one bot turn; it does not proxy the client's own messages into an existing Codync thread, and a
  browser client must be served from the host's own origin (or call it server-to-server, with no `Origin`), as
  with every other local API route.

## The provider: a bot on a remote AG-UI endpoint

The `agui` backend ("AG-UI agent") is built into the host. It has no CLI to install: the host starts its own
`codync-host agui-acp` process, which speaks ACP to the bot over stdio and HTTP to the endpoint. The adapter is
the same seam every harness uses (`start_agent` → ACP), so the bot's chat, approvals, routine runs and fallbacks
work unchanged.

**Configuration**

1. Create or edit a bot and set **Agent** to **AG-UI agent**.
2. Open **Extensions** on that computer, choose **AG-UI agent**, and save:
   - **Endpoint URL** (`CODYNC_AGUI_URL`): the URL that accepts `POST` with a `RunAgentInput` and answers with
     `text/event-stream`. HTTPS anywhere; HTTP only on `localhost` or a loopback IP, and no credentials, query or
     fragment in the URL. The same rules as the local voice server.
   - **Access token** (`CODYNC_AGUI_TOKEN`, optional): sent as `Authorization: Bearer`.
3. The values are kept in the host vault (`setAgentEnv`, slot `agent-env:agui`) and handed to the adapter's
   process on every start; they never reach a client. Saving them starts a running bot's adapter again, so a
   change reaches the next turn.

The adapter advertises those two fields as an `env_var` ACP auth method, which is why they appear in the agent's
own settings. If no URL is saved, the first turn fails with a message naming the setting.

**What the adapter maps**

| AG-UI event (from the endpoint) | Codync (ACP) |
| --- | --- |
| `TEXT_MESSAGE_CONTENT` | `session/update` `agent_message_chunk` |
| `REASONING_MESSAGE_CONTENT`, `THINKING_TEXT_MESSAGE_CONTENT` | `session/update` `agent_thought_chunk` |
| `TOOL_CALL_START` | `session/update` `tool_call` (`toolCallId`, title, pending) |
| `TOOL_CALL_ARGS` (accumulated) + `TOOL_CALL_END` | `session/update` `tool_call_update` with `status: in_progress` and the parsed `rawInput` |
| `TOOL_CALL_RESULT` | `session/update` `tool_call_update` with `status: completed` and the content |
| `CUSTOM` whose name is `codync.approval` or contains `approval` | `session/request_permission`, with the endpoint's options (or Allow/Deny) |
| `RUN_FINISHED` | the ACP prompt returns `stopReason: end_turn` |
| `RUN_ERROR` | the ACP prompt returns a JSON-RPC error with the endpoint's message |
| `STATE_*`, `RAW`, and other `CUSTOM` events | ignored |

Each run posts the cumulative conversation (`messages`) with a fresh `runId` and the ACP session id as
`threadId`. After an approval, the answer goes back as a `tool` message (`{"approved":bool,"optionId":…}`) in a
**follow-up run of the same ACP turn**, up to eight approvals per turn. The endpoint must therefore accept more
than one run on one `threadId`.

**What the provider does not cover**

- Tools declared in `tools` are not executed by Codync: the remote endpoint must run its own tools and report
  their results. Codync's own MCP tools cannot be called by the remote agent.
- The remote agent runs with the endpoint's own model; the bot's model, reasoning effort and Fast mode settings
  are not sent.
- The adapter does not resume a remote conversation after a host or endpoint restart (`loadSession` is not
  advertised): the next turn starts a new thread with the bot's current context in the first message.
- The `state` document is not sent back by the endpoint to Codync; it stays a client-side concern.

## Tests

`host/src/agui/*` tests cover:

- the request parser (newest user message, content parts, generated ids);
- `Mapper`: streamed text start/delta/end, a new segment closing the old one, tool start/args/end/result,
  approval options and answer body, `RUN_FINISHED`, `RUN_ERROR`, and a stopped run;
- SSE framing (`data:` line, blank line, escaped newlines) and chunking at character boundaries;
- the endpoint validator (HTTPS, loopback HTTP, no credentials/query/fragment);
- the remote SSE parser across arbitrary chunks, CRLF, comments, multi-line `data:` and `[DONE]`;
- the shim's initialize auth methods and approval-option normalization.

There is no local HTTP round-trip test: the existing API tests exercise the router's middleware without an
agent, and this route needs a running bot actor to produce a turn. It is checked by hand with an AG-UI client.
