# Modules

A **module** is something a person adds to Codync beyond what it ships with: a folder with a signed manifest, a
program the host runs in its own process, and the tools and screens it declares. It is the one extension point the
host has.

Status (2026-11-15): built on 2.53.0, on every client.

The idea comes from [agentes.chat](../features/agentes-chat.md) (ADR 0002), where a module is an object inside the
server process that swaps adapters for a Cloud edition. Codync has one edition and one person, so a module here is
something the person installs: signed, beside the host rather than inside it, and it asks before it acts. The code
is ours.

## What a module is

```
weather/
  module.json      the manifest: identity, entry, permissions, limits, tools, views, file hashes, publisher key
  module.sig       the Ed25519 signature over module.json's canonical bytes
  main.js          whatever the entry runs
  data/            the module's own writable folder, never signed
```

`module.json`, camelCase, `deny_unknown_fields`:

```json
{
  "format": "codync-module",
  "contract": 1,
  "id": "com.example.weather",
  "name": "Weather",
  "version": "1.2.0",
  "description": "Forecast for the bot's city.",
  "symbol": "cloud.sun",
  "entry": { "command": "node", "args": ["main.js"] },
  "permissions": { "tools": ["forecast"], "events": ["botStatus"], "network": ["api.open-meteo.com"], "files": "read" },
  "limits": { "callsPerMinute": 60, "timeMs": 20000, "memoryMb": 256 },
  "tools": [{ "name": "forecast", "description": "Tomorrow's weather.", "inputSchema": { "type": "object", "properties": {} } }],
  "views": [{ "id": "today", "title": "Today" }],
  "files": [{ "path": "main.js", "sha256": "…" }],
  "publisher": { "name": "Example", "key": "…base64 Ed25519 public key…" }
}
```

The signature covers the manifest written one way (the host's own serialization, so whitespace never matters), and
every listed file is hashed. The host refuses a module whose signature doesn't verify, a file whose hash differs, a
file the manifest doesn't list, or a `contract` above the one it speaks ("needs a newer Codync"). `module.sig` and
`module.json` are never listed; `data/` is the module's own.

## The process contract

One JSON-RPC 2.0 object per line on stdin and stdout. The host starts the process on the module's first call and
keeps it while it answers.

| Host → module | Params | Result |
|---|---|---|
| `initialize` | `{contract, hostVersion, moduleId, botFolder, files}` | `{tools, views}` (checked against the manifest) |
| `tool` | `{name, arguments, botFolder}` | `{content: [{type: "text", text}], isError?}`, MCP's own shape |
| `view` | `{id, botFolder}` | `{title, sections: [{title, rows: [{label, value}]}]}` |
| `event` (notification) | `{name, body}` | — |
| `shutdown` | — | `{}` |

A module may notify the host with `{"method": "event", "params": {name, body}}`, only for names it declared and the
owner approved. Those leave the host as `{type: "moduleEvent", module, name, body}` on the event stream.

`botFolder` is the bot's folder, passed only when the owner approved `files: read` or `write`. The environment is
scrubbed (`PATH`, `HOME`, `USER`, `LANG`, `CODYNC_MODULE_ID`, `CODYNC_MODULE_DATA`, `CODYNC_MODULE_CONTRACT`): no host
token, no data directory. A module runs as the person, like a connector; approvals and quotas are what bound it.

## What it cannot do

- **Change anything by itself.** No settings, no approvals, no host API: the only door is the tools the host calls
  for it, each checked against the owner's approval.
- **Act before approval.** A fresh install is off with nothing approved; its tools aren't even listed to bots.
- **Reach a tool it didn't declare.** A manifest that permits an undeclared tool is refused at load.
- **Run past its quotas.** Time, calls per minute and memory are held by the supervisor; a call that overruns stops
  the process and says why, and the next call starts a fresh one.
- **Hear an event it didn't declare.** The host forwards only `messageAdded` and `botStatus`, and only to modules
  that asked for them and were approved.

Not enforced, and documented as such: `network` and `files` are declared and shown, not firewalled. There is no OS
sandbox per platform yet (see [not built](#not-built)).

## Permissions and approval

The owner approves a subset of what the manifest asks for; `approveModule` cuts the answer down to the manifest's own
lists, so a client can never grant more than the author declared. Nothing approved means nothing listed, and a tool
that isn't approved answers the agent "isn't approved for this module".

## Lifecycle

| Method | Body | Answers |
|---|---|---|
| `modules` | — | `{modules: [row]}` |
| `installModule` | `{folder}` or `{bundle}` | `{module, modules}`; off, nothing approved |
| `updateModule` | `{id, folder}` or `{id, bundle}` | `{module, modules}` |
| `uninstallModule` | `{id}` | `{modules}`; stops it and deletes its folder |
| `setModule` | `{id, enabled}` | `{modules}` |
| `approveModule` | `{id, permissions?}` | `{modules}`; `permissions` absent approves all it asks |
| `exportModule` | `{id}` | `{name, bundle}` |
| `moduleView` | `{id, view, botId?}` | `{view}` |
| `moduleTools` | `{id}` | `{tools}`; what the bot's MCP server lists |
| `moduleCall` | `{moduleId, name, arguments, botId}` | MCP tool result |

Installing and removing touch this computer's disk, so only its own apps ask; a paired phone may list, turn one on or
off, approve and read a screen. An update signed by the same publisher key keeps the approval; another key turns the
module off with nothing approved.

A **bundle** is one file another computer installs: `{format: "codync-module-bundle", version: 1, manifest,
signature, files: [{path, content}]}` (content base64). Importing re-verifies the signature and every file's hash
before anything is written.

## Tools in a bot

Each module a bot may call adds one MCP server to that bot's session, `module-<id>`, started as
`codync-host mcp module --bot <id>`. It lists the manifest's approved tools without waking the module, and every call
goes through `moduleCall`, which checks approval and quotas and then writes one line to the module's process. A
module turned off, or one whose files stopped verifying, offers nothing.

## Screens

A module's view is rows of text, not HTML: the host normalizes it (at most 20 sections, 200 rows each) and every
client draws it with its own components.

| Client | Where |
|---|---|
| Desktop | Extensions › Modules: install a folder or a `.module.json`, approve, switch, screens, save, remove |
| Terminal | Extensions › Modules: `↵` on/off, `^a` approve, `^v` screen, `^s` save, `^i` install from a folder, `^x` remove |
| Android | Extensions › Modules: install from a file, approve, switch, screens, save, remove |
| iPhone | Extensions › Modules: install from a file, approve, switch, screens, save, remove |

## Limits

| Quota | Manifest asks | Host allows |
|---|---|---|
| Calls per minute | up to 600 | 1–600 |
| Time per call | up to 120 s | 1–120 s |
| Memory | up to 2 GB | 16 MB–2 GB |

A module can ask for less than the ceiling, never more. Memory is read every two seconds where the platform reports
it (`/proc` on Linux, `ps` on macOS, `tasklist` on Windows); past the cap the process is stopped.

## Tests

- the manifest's own checks: an undeclared tool permission, an unknown event, a path that leaves the folder, a newer
  contract, limits that only come down;
- a signed folder verifies, a changed file and an unlisted file do not, and a folder without `module.sig` isn't a
  module;
- install, approve (never more than declared), export and import round trip, a tampered bundle is refused, and an
  update from another key starts over off;
- the sample module (`host/src/bin/module-demo.rs`) answers a tool and a view, a tool nobody approved never reaches
  it, a module that never answers is stopped and the next call starts a fresh process, and one that never closes is
  killed when it is stopped.

## Not built

- An OS sandbox per platform (Seatbelt, bubblewrap, a Windows job object): a module runs as the person, in its own
  process group, held by quotas.
- A registry of modules, versions or signatures beyond the publisher key in each manifest.
- Firewalling the declared `network` hosts, or a filesystem view narrower than the bot's folder.
- Modules that add routes to the API, replace host behaviour, or ship their own UI beyond rows.
