# Approvals

An agent asks before some tool calls (ACP `session/request_permission`). The host answers it
itself or shows an approval card, and it decides whether a card can still be answered. The code is
in `host/src/agent/bot/updates.rs` (`on_permission`, `answer_permission`),
`host/src/agent/approval_rules.rs` and `respondPermission` in `host/src/api/bots.rs`.

## Rules

A bot's `approvalRules` (part of its config, set with `updateBot`) are checked in order, before
the bot's general setting (`permission`: `ask` or `auto`). The first rule that matches decides:

| `decision` | What happens |
| --- | --- |
| `allow` | The host picks the agent's allow option (`allow_once`, then `allow_always`, then its first option). No card. |
| `ask` | A card, even for a bot set to approve everything. The card's `rule` names the pattern. |
| `deny` | The host picks the agent's reject option (`reject_once`, then `reject_always`), or cancels the request. A notice in the chat names the rule. |

A rule is `{pattern, kind?, decision}`. `pattern` is matched against the request's command (a shell
tool's `rawInput.command`) or else its title, as a whole, ignoring case; `*` stands for any text.
`kind`, when set, limits the rule to one tool kind (`execute`, `edit`, `read`, `fetch`…). Requests
no rule matches follow `permission` as before. A bot keeps at most 100 rules of up to 500
characters; `updateBot` refuses more.

An `allow` rule never approves a command that runs more than one (`;`, `&&`, `||`, `|`, `` ` ``,
`$(`, `>`, `<`, a newline), so `git status*` doesn't let `git status && rm -rf ~` through. Such a
command goes on to the next rules and then to the card. `deny` and `ask` rules still match it.

Bots can't set their own rules: only `updateBot` changes them, and neither the bot's profile tools
nor the fleet tools pass them on. The desktop app, the Android app and the terminal edit them (section "Editing the
rules"). Rakazo's automated review of
low-risk requests isn't implemented: no model approves on the user's behalf.

## Editing the rules

The rules are in the bot's settings (desktop and Android: under Permissions; terminal: the palette's "Approval rules of
this bot…"). Each rule has a decision (allow, ask, deny), an optional tool kind, and a pattern; the list is ordered
(the first match decides) and can be added to, edited, reordered and deleted. Blank patterns are dropped when saved. The
editor shows the warning that an allow rule never approves a chained command.

**Try a command.** `testApprovalRule {botId, command, kind?, rules?}` returns `{evaluation}`: what the bot would do with
the command, by the same `approval_rules::evaluate` a real request goes through, so the tester never copies the logic.
`outcome` is `forbidden` (with the blocklist `pattern`), `rule` (`index`, `decision`, `pattern`) or `setting` (the
bot's `permission`), plus `chained` and `allowPassedOver` (an allow rule that matched but was skipped because the
command runs several). `rules` tests a list that is not saved yet; without it the bot's saved rules are used.

## Forbidden commands

Some commands no bot may run, in any profile and with any approval setting, `auto` and the future `Full`
profile included (`host/src/agent/blocklist.rs`): wiping the root or the home folder (`rm -rf /`, `rm -rf ~`,
`Remove-Item -Recurse -Force C:\`, `rd /s /q C:\`), `sudo`, `mkfs`, `dd if=`, shutting down, formatting, changing
the execution policy, `net user`, `reg delete`… They are checked before the user's rules and before the bot's
setting. A match answers the agent's request with its reject option (a cancel when it has none) and writes a notice
that names the pattern: "Blocked command ("rm -rf /"), never allowed". The same list stops a background `shell_job`
and a terminal agent's hook (the terminal is told to refuse).

The command is split at `;`, `&`, `|`, newlines, brackets and backticks, and each part is checked after the shell
wrappers around it are opened (`env`, `FOO=1 …`, `bash -c`, `cmd /c`, `powershell -Command`, up to three deep), so a
chain or a wrapper doesn't hide it, while words inside a quoted message (`git commit -m "rm -rf /"`) are not a
command. The list is Mercury Agent's (MIT), with one deliberate change: its `/*` and `C:\*` use `*` as "anything",
which would refuse `rm -rf /tmp/build` and `rd /s /q C:\Users\me\proj\build`. Here a bracketed `[*]` is a literal
star, so only the root, the home folder and their contents are refused. The matcher is the same one the user's rules
use (`approval_rules::glob`); `?` means one character only in this list, not in the user's rules.

## Cards nobody answers

A card left `pending` pushes one reminder after 4 hours (`NeedsInput`, with the card's own options) and
closes as `cancelled` after 24 hours, and the agent gets a cancel. The reminder is written into the card
(`reminded: true`), so it cannot repeat, even after a restart. Code: `host/src/agent/bot/approval_age.rs`; the
clock is injected in its tests.

A human takeover request (`request_handoff`, `host/src/chat/takeover.rs`) pushes `NeedsInput` once as it is
posted (`takeoverRequest.notified`), and a 60 second sweep turns it into `expired` after 30 minutes, with the
chat line rewritten as "Takeover request expired". Desktop and Android show an expired card closed and keep it
out of the pending decisions; the terminal shows the rewritten line. iOS has no takeover card. This build's
request does not hold the bot's computer: the remote screen's own takeover (`screenTakeover`) is the person's
choice and is not touched.

## Answering a card

The card is the effect's key: the first decision is the one that happens.

- `respondPermission` with the choice the card already has (a second device, a retry) returns
  `{entry}` as it stands and changes nothing.
- A different choice on an answered card is refused: "This approval was already answered with
  another choice".
- A card that is `cancelled` (its turn ended or was stopped) is refused: "This approval is closed".
- A card whose agent no longer waits on it (its process went away) is closed when someone answers
  it, instead of looking answered.
- Anything that isn't a permission entry is refused.

Two answers that arrive at the same moment both see a pending card; the bot's actor applies the
first and drops the second, so only one reaches the agent.

Ideas from rakazo's `action-approval.ts`, `approval-effect-key.ts` and `answerable-ask.ts`
(Apache-2.0); no code copied. Rakazo's channel receipts and message commands need messaging
channels, which Codync doesn't have.
