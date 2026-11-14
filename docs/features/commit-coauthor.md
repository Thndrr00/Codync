# Commit co-author

OpenBot ends its agents' git commits with a co-author trailer. Codync can do the same, per bot, when the
person turns it on: **Settings → Agent → Credit Codync in commits** in the desktop app, and the same switch in
the bot's settings on the iPhone. It is off by default and stays off until someone turns it on (OpenBot's
default is on; Codync does not change the commits a bot writes without being asked).

## The exact trailer

```text
Co-authored-by: Codync <bot@codync.dev>
```

It is one `git` trailer (`Token: value`), on a line of its own after a blank line at the end of the message.
The person stays the commit's author. The text lives in one place in the host:
`host/src/chat/commit_coauthor.rs` (`TRAILER`), so a test holds it to this page.

## How it reaches the agent

Codync does not rewrite commit messages itself: the bot's agent runs `git commit`, and Codync cannot see every
commit a CLI writes (a rebase, a force-amend, a commit an MCP server makes). The setting adds a paragraph to
the bot's profile (`chat::context::render`), which the agent receives as its system prompt or first message, as
its other instructions do:

> Commits: end the message of every git commit you create or amend, also with a message from the user, with this
> trailer once, on a line of its own after a blank line. Keep the user as the author, and add no other co-author
> line.

`commit_coauthor::section(false)` is empty, so a bot with the setting off gets nothing and its profile is
unchanged. A bot stored by an older host reads the field as off (`serde(default)`), and a client that doesn't
know the field leaves it as it is.

`commit_coauthor::append` is available for the few places where Codync writes a commit message itself; it adds
the trailer once, after a blank line, and leaves a message that already ends with it unchanged.

## Limits

- A commit whose message an agent writes without reading its profile (an agent that skips instructions, or a
  manual `git commit` a person runs in the same terminal) has no trailer. Codync reports no error for that; the
  setting is an instruction, not a rewrite of `.git/hooks`.
- The trailer is the same for every bot. A message that already has a co-author line simply gets one more only
  if it lacks Codync's exact line.
- Groups have no agent of their own, so the switch applies to agent bots. The terminal UI and Android show no
  switch in this pass.

## Tests

`host/src/chat/commit_coauthor.rs` fixes the exact trailer text, adds it once after a blank line, keeps a
message that already has it, and shows the profile paragraph only when the setting is on.
`host/src/chat/context.rs` checks that `render` includes the trailer only for a bot with `commitCoauthor`.
