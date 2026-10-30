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
nor the fleet tools pass them on. No client edits them yet. Rakazo's automated review of
low-risk requests isn't implemented: no model approves on the user's behalf.

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
