# Proposed skills

When a turn (or a routine run) ends well after using **three or more different kinds of tool** (the ACP kind; for commands,
the program: `cargo` and `git` count as two), the bot's own agent drafts a skill from what it did: a name, when to use it
and the steps. The draft waits in an inbox. **A proposal is never an active skill.**

- **Rules to hold the cost down:** no draft while the bot's token budget has reached its warning level; at most one draft
  per bot per hour; none for a procedure already waiting or dismissed. A procedure's fingerprint is its set of tool kinds
  plus the first eight words of the request, so the order of steps does not matter.
- **API (`bots.rs` → `chat/skill_proposals.rs`):** `skillProposals {botId}`, `editSkillProposal {botId, id, name?,
  description?, instructions?}`, `approveSkillProposal {botId, id}` (saves it with the `save_skill` path, so it is a skill of
  that bot only, with a notice in its chat), `dismissSkillProposal {botId, id}` (its fingerprint is remembered). Each
  answers with the inbox as it is now.
- **Storage:** a list per bot in the key-value store (20 waiting at most, 200 dismissed fingerprints).
- **Approving while the agent is busy** fails fast with the fixed, retryable message "The agent is busy; try again when it finishes": the draft stays in the inbox and no skill is saved, so the button doesn't spin on the agent's queue. The draft leaves the inbox only after the save succeeds (OpenBot 0.77.0; pure `approve_outcome`).
- **Screens:** the inbox sits under the Memory tab of the bot's details on desktop and Android ("Proposed skills (n)", one card
  per proposal with Approve, Edit, Dismiss); in the terminal "Skills this bot proposes…" (`a` approve, `x` dismiss; editing
  is done on desktop or the phone). The count is in the inbox title. iPhone: not done.
