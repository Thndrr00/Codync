# Server settings: Personality, Claude Code setup and remote desktop

Three settings belong to a computer rather than to one bot. They live in the host's `kv` table as
one JSON document (`server.settings`, `host/src/server_settings.rs`), and every client reads and
writes them through `serverSettings` / `setServerSettings`. A missing, unreadable or newer document
reads as the defaults, field by field, so one unknown value from a newer app never resets the rest.

## Personality (updates between steps)

`responseStyle` is `natural` (the default) or `classic`.

- **Natural** adds one section to every bot's instructions: write a short line when a finding changes
  the plan or before a long step, say what you understood and check first when you use a tool, and
  answer with the verdict first, explaining a technical term in a few words and saying what could not
  be checked. The agent's messages still stream into the chat as before; the style changes what the
  agent chooses to say between steps. Ported from OpenBot 0.64's "Natural" response style.
- **Classic** adds nothing: the agent's own text, as it comes.

The section is rendered by `chat/context.rs` from `Settings::style_section`, so it reaches every
harness through the same instructions. It is server-wide; a bot's own soul, persona and heartbeat
remain its separate, per-bot Personality (`agent::personality`).

## Use my Claude Code setup

`claudeUserSetup` is **off by default**. Claude agents start with `settingSources: []`, so they do not
load the person's `~/.claude` CLAUDE.md, skills, agents, hooks or plugins, and each turn sends a
shorter prompt. Turned on, `claude_options` sends `settingSources: ["user", "project", "local"]`.
The person's sign-in settings in `~/.claude/settings.json` (`env`, `apiKeyHelper`) reach the agent
through its environment either way, and the files of the bot's workspace still load. The switch is
applied when a session starts or resumes, so a change reaches the bot's next new session.

## Remote desktop policy

`remoteControl.access` is `everyone` (default), `admins` or `owner`; `remoteControl.askOnHost` is
off by default.

- `everyone` and `admins` both allow every paired device. Codync has one owner and no member roles
  yet, so "owners and admins" cannot mean anything narrower; the value exists for the day accounts
  have roles.
- `owner` allows only this computer's own app: `screenPrepare` from a paired device is refused with
  "This computer only allows remote control from itself."
- With `askOnHost` on, a device's `screenPrepare` posts a question to this computer
  (`Screen::state`'s `consent`, with a 30 s `expiresAt`) and waits in the same request. The person
  here allows or denies it (`answerScreenRequest`, local only); nobody answering within 30 seconds
  ends the request with an error, and the session does not start.
- `screenStopAll` (local only) closes every viewer session through the screen helper without turning
  Remote screen off, so a later session can start again under the same policy.

Phones show the wait as "Connecting…" (their `screenPrepare` timeout is 45 s) and the host's reason
when it is denied. The terminal UI has no video viewer and is unaffected.

## Related

- [Chat history actions](chat-history.md)
- [Remote screen](remote-screen.md)
- [Agent accounts](agent-accounts.md)
