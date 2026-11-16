# Automatic learning

A bot can learn a durable fact or preference from **what the person tells it**, without being asked to remember. The
fact is proposed for review and is written to the bot's memory only when the person approves it (or, if configured,
after the person has said it a set number of times).

## What is learned, and what is never read

Only the person's own messages in the bot's own chat are read. Group chats, routines, the bot's replies and outside
text never propose anything.

A sentence becomes a candidate when it is:

- **A correction**: "No, use pnpm, not npm", "Actually, the tests run before the commit", "I said tabs, not spaces".
- **A stated preference**: "I always want the tests run before a commit", "From now on reply in British English", "I
  prefer short answers".
- **A repeated instruction**: the same instruction in a later turn (politeness such as "please" is ignored when
  comparing), with at least two sayings.

Questions, one-off tasks, code blocks, URLs, email addresses, long numeric identifiers and anything that looks like a
password, API key or token are left alone. Facts are 12–240 characters and at most 40 words.

## The proposal and the review

A proposal waits in the bot's **Learned preferences** inbox with the fact, why it is there (correction, preference or
how many times it was said) and the message it came from. Approving writes one line to the bot's `preferences.md`
note with the belief header `type: feedback`, `confidence: high` and today's date. Dismissing remembers the fact's
fingerprint, so it is not proposed again. Saying the same fact again updates the hit count instead of making a second
card.

`autoApproveAfter` (0 = off, up to 10) saves a fact without review once it has been said that many times. The chat
gets one "I learned: …" notice; the memory screen has the note and its history, as for any memory change.

## Storage and API

- A list per bot in the key-value store: 20 waiting at most, 200 dismissed fingerprints.
- `learning {botId}` answers with `{enabled, autoApproveAfter, maxAutoApproveAfter, note, proposals}`.
- `approveLearning` / `dismissLearning {botId, id}`, `saveLearningSettings {botId, enabled?, autoApproveAfter?}` each
  answer with that inbox as it is now.
- The bot's `learning` setting is `{enabled (default true), autoApproveAfter (default 0)}`; it is saved with
  `updateBot`.

A fact already in `preferences.md` or `MEMORY.md` is neither proposed nor written twice. An approved fact is the
person's, so a bot may correct or forget it later under the usual memory rules.

## Screens

| Client | Where |
|---|---|
| Desktop | Bot details → Memory → **Learned preferences (n)**: Remember this, Dismiss, learning on/off, review threshold |
| Android | Bot details → Memory → the same card and actions |
| iPhone | Bot details → Memory → **Learned preferences**: the same card and actions |
| Terminal | Command list → **Learned preferences**: `a` remember, `x` dismiss, `t` learning on/off, `,` `.` threshold |

## Validation

`host/src/learning/detect.rs` tests correction detection, preference detection, repetition, the rejection of code,
URLs, emails, secrets and account numbers, and fingerprints. `host/src/learning/tests.rs` covers a proposal waiting
without touching memory, approval writing one feedback fact and clearing the inbox, dedupe of a repeated correction,
dismissal staying dismissed, automatic saving after the configured repeats, turning learning off, and clamped
settings. The host was not built or run in this change; the client screens were not exercised.
