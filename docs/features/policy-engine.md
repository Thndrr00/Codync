# Policy engine

A policy is a named rule for **this computer**: a CEL expression, an effect (`allow` or `deny`) and a
message. The rules run at the same seam as the approval rules and the forbidden commands, before a
bot's own settings, so what a bot may do is decided by the computer rather than by each bot's editor.
The code is `host/src/policy/` (engine, variables, presets, API) and `host/src/store/policies.rs`
(storage and versions).

Ideas from CopilotKit/OpenBot 1.0's policy engine: named rules, deny before allow, a per-computer list
and a what-if. The expression language, storage and seam are Codync's own.

## Effects

| Effect | What a match does |
| --- | --- |
| `deny` | The agent's request is refused: it gets its reject option (a cancel when it has none), the chat gets a notice naming the rule and its message, and an audit line is written. |
| `allow` | The request is approved: the host picks the agent's allow option (`allow_once`, then `allow_always`, then its first option), as an `allow` approval rule does. |

`deny` rules are all tried before any `allow` rule, whatever order the list is in.

## Precedence

For one tool call, in order:

1. **Forbidden commands** (`host/src/agent/blocklist.rs`): a command no bot may run, in any profile.
   Refused, naming the pattern. A policy cannot allow it.
2. **Policy `deny` rules**, in the order the person arranged them. The first match refuses.
3. **Policy `allow` rules**, in that order. The first match permits.
4. **The bot's approval rules** (`docs/features/approvals.md`), first match first.
5. **The bot's Approvals setting** (`ask` or `Auto`).

Two things hold across that list:

- A **denial always wins**. A policy `allow` never overrides a forbidden command or one of the bot's
  own `deny` approval rules; it decides between "approved" and "a card".
- With **no policy switched on**, steps 4 and 5 decide exactly as they did before policies existed.

### A list that allows becomes an allowlist

- At least one enabled `allow` rule and no rule matches → **refused**: "no policy on this computer
  allows it". Nothing falls through to the approval rules, because a list that permits must name what
  it permits.
- Only `deny` rules (or none) and no rule matches → steps 4 and 5 decide. A deny-only list narrows what
  may happen; it doesn't take approvals away.
- No enabled rule at all → steps 4 and 5 decide.

To let everything else through to the approval cards, save a last `allow` rule whose expression is
`true`. That is a rule like any other: it is listed, editable and removable.

### Fail closed

- A rule that **doesn't compile** refuses the call, naming the rule and the compiler's first line.
- A rule that **doesn't evaluate**, or evaluates to something that isn't `true` or `false`, refuses the
  call, naming the rule and the error.
- A rule is **tried before it is saved**: the host compiles it and runs it against a sample request for
  every initiator (`person`, `routine`, `channel`, `network`). A rule that won't run is not saved, and
  the editor shows why.
- If the saved list **can't be read** (a database error), the call is refused rather than allowed.

## Variables

Every variable is a string, and every one is always present: an empty string means the call doesn't
have it, so `tool.name == "shell"` never fails on a call that isn't a shell tool.

| Variable | Meaning |
| --- | --- |
| `tool.name` | The agent's tool name. An ACP request carries none of its own, so its title's first word stands in (`shell`, `run`); a connector tool named `mcp__server__tool` is read as that connector and tool. |
| `tool.kind` | `execute`, `edit`, `delete`, `move`, `read`, `search`, `fetch`, `think`, `other` (inferred from the tool name when the agent doesn't say). |
| `intent` | What the call says it does: its title, or the what-if's `intent`. |
| `bot.id`, `bot.name` | The bot whose agent is asking. |
| `actor.id`, `actor.role` | The person whose bot it is, and `owner`, `admin` or `member`. |
| `command` | The shell command, when the call has one. |
| `file.path`, `file.op` | The file the call is about, and `read`, `edit`, `delete` or `move`. |
| `mcp.server`, `mcp.tool` | The connector and its tool, for a connector tool. |
| `page.url`, `page.host` | The page, for a browser action. |
| `initiator.kind` | `person`, `routine`, `channel` or `network`: who set the turn off. A group or channel post is `channel`; a scheduled run is `routine`; a message from another computer is `network`; everything else is `person`. |
| `key` | The key a keyboard action presses. |

Examples:

```cel
tool.kind == "execute" && !command.startsWith("git status")
file.path.startsWith("~/work/") && file.op in ["edit", "delete"]
bot.name == "Release bot" && intent.matches("(?i)publish")
mcp.server == "github" && mcp.tool in ["create_issue", "merge_pull_request"]
initiator.kind != "person" && tool.kind == "execute"
```

## The expression language

The parser and interpreter are [cel-rust](https://github.com/cel-rust/cel-rust): the `cel` crate,
**version 0.16.0** (`host/Cargo.toml`), with the standard library plus its `regex` feature for
`matches()`. A rule is compiled into the standard environment; the interpreter's own extensions are not
registered.

Supported, as CEL defines them:

- `&&`, `||`, `!`, `?:`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `in`
- strings: `+`, `size()`, `startsWith()`, `endsWith()`, `contains()`, `matches()`
- `int()`, `double()`, `string()`, `bool()`
- lists `[a, b]` and maps `{"k": v}` (variables' groups are maps, so `tool.name` is `tool["name"]`)
- the macros `has()`, `all()`, `exists()`, `exists_one()`, `map()`, `filter()`

Not available, so a rule using them won't compile and is refused at save time:

- timestamps, durations and time zones (the crate's `chrono` feature is off)
- protobuf types, custom functions and user-defined macros
- backreferences in `matches()`: the `regex` crate's syntax (RE2-like), not PCRE

`matches()` takes a pattern as written in CEL, so a word boundary is `"\\b"` in the rule's text.

## Limits

- At most **100** rules per computer; a name of 1–80 characters, an expression of 1–500, a message of
  up to 200.
- The newest **20** versions of a rule are kept (`policyVersions`).
- One rule is one expression: no loops of its own. The comprehension macros run over the small lists the
  variables carry.
- A rule is compiled on each evaluation. A permission request is a human-scale event, so this keeps a
  rule's text and what runs identical; nothing is cached that could go stale.

## Storage and versions

`policies` holds the rules in the person's order, with `enabled`, `version` and timestamps;
`policy_versions` keeps what each save said (newest 20 per rule). A save increments the version, so an
editor can show what changed. Disabling a rule is a save (one version up); removing it takes its
versions with it. `schema` migration is not needed: the tables are created on open (`policies.sql`).

## Simulator (what-if)

Nothing in either call acts.

- `previewPolicy { tool, command?, kind?, intent?, initiator?, botId?, filePath?, fileOp?, mcpServer?,
  mcpTool?, pageUrl?, key?, policies? }` answers one proposed call against this computer's rules (or the
  `policies` the editor sends): `{ preview }` with `decision` (`allow`, `deny` or `unset` — no policy
  decides), `source` (`policy`, `noMatch`, `error`, `unset`), the deciding `rule` (`name`, `effect`,
  `index`, `message`), `allowlist`, `refusal`, `rules` (every enabled rule's own answer, in the order it
  is tried: its name, effect, whether it matched, or the error that made it refuse), `variables` (the
  resolved snapshot) and `explanation`, a sentence ending "Nothing ran."
- `previewApproval` (the bot's own what-if) now returns a `policy` object with the same shape, and its
  `decision`, `source` and `explanation` follow the policy when the policy decides. Its `policies` input
  tests an unsaved list.

The snapshot is **redacted**: a command's credentials are masked (`--token abc`, `API_KEY=abc` →
`[redacted]`), a page's query string and fragment are dropped, the home folder is shortened to `~`, and
every value is capped at 200 characters. The rules read the request as it is; only what a person is
shown is masked.

## Presets

Three rules the editor offers as starting points. They are ordinary rules: nothing is enforced until
one is saved, and then it can be edited or removed like any other (`policies` returns them; a save
creates the rule).

| Preset | Effect | Rule |
| --- | --- | --- |
| Read-only | allow | `tool.kind in ["read", "search", "fetch", "think"]` — this makes the list an allowlist, so everything else is refused. |
| No shell | deny | `tool.kind == "execute"` |
| No sends | deny | the intent or tool name says send, post, publish, email, mail, message, tweet or reply |

## API

| Method | Body | Answer |
| --- | --- | --- |
| `policies` | `{}` | `policies`, `presets`, `variables` (name and what it means), `limits`, `enforcing`, `allowlist` |
| `savePolicy` | `{policy}` (`id` empty adds one) | `{policy}` with its `version`; refuses with the compile or evaluation error |
| `setPolicyEnabled` | `{id, enabled}` | `{policy}` |
| `removePolicy` | `{id}` | `{removed}` |
| `policyVersions` | `{id}` | `{versions}`, newest first |
| `previewPolicy` | see above | `{preview}` |

Only the computer itself and its owner's (or an admin's) paired devices may save, switch or remove a
rule; any paired device may read the list and ask the what-if. Bots can't set policies: no bot tool
passes them on. Saving, switching and removing a rule are audit lines; a refusal during a run is one
too, naming the rule and the bot.

## Clients

- **Desktop**: Settings → **Approvals & policies** (under This computer): the list, add/edit/remove,
  switches, the presets, the what-if with the variable snapshot, and the variables a rule may use. A
  bot's own what-if ("What would happen?") shows the policy's answer first.
- **iPhone** and **Android**: Settings → **Approvals & policies** (the phone's Settings tab): the same
  list and editor, the presets, and the what-if.
- **Terminal**: the palette's **Policies of this computer…**: `a` adds the next preset, `n`/`e`/`m` edit
  the name, expression or message, `space` switches a rule, `d` removes it, `T` asks the what-if about a
  shell command.
- Refusals read the same everywhere, because the host writes the notice: "Refused by the policy «name»:
  message".
