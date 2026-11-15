# Import an agent

Settings → Import an agent (desktop): choose the folder of a Claude Code, OpenClaw or Hermes agent, or any folder with an `AGENTS.md`,
review what it holds and create a bot from it.

- **Formats.** Claude Code: `CLAUDE.md` and `.claude/CLAUDE.md` (instructions), `.claude/skills/*/SKILL.md`,
  `.claude/commands/*.md` and `.claude/agents/*.md` (each a skill, named after its file); settings files
  (`settings.json`, `settings.local.json`, `.mcp.json`) are never read. A folder with a `CLAUDE.md` is Claude Code even
  when it also has an `AGENTS.md`. It is the first source added after the original two because it is the most asked
  for; another source is one reader branch next to it, with its own file list and nothing else. OpenClaw: `SOUL.md`, `AGENTS.md` (instructions), `IDENTITY.md` (name
  and description), `MEMORY.md`, `USER.md`
  and `memory/*.md` (memory), `skills/*/SKILL.md`; runtime files (`TOOLS.md`, `HEARTBEAT.md`, `BOOTSTRAP.md`, `BOOT.md`) are
  listed as not imported. Hermes: `SOUL.md`, `AGENTS.md`, `HERMES.md`, `.hermes.md`, `memories/MEMORY.md` and `USER.md`
  (entries separated by a `§` line become one paragraph each), and `skills/`, without the skills Hermes installed itself
  (`skills/.bundled_manifest`). The home's `workspace/` folder is read as the workspace.
- **Credentials never travel.** Settings and secret files (`.env`, `auth.json`, `openclaw.json`, `settings.json`,
  `settings.local.json`, `.mcp.json`, `credentials`, `*.pem`,
  `*.key`...) are named in "Not imported" and never read. Text shaped like a key (OpenAI/Anthropic/OpenRouter `sk-`, GitHub,
  Slack, AWS, Google, GitLab, Hugging Face, Stripe, Telegram, private keys, `Bearer` tokens) is shown as omitted, cannot be
  ticked and is never sent. The host checks again: `importAgentMemory` leaves out any fact with that shape.
- **Preview.** Each file or skill is a switch; template files an agent shipped with start unticked. Nothing is created until
  Import.
- **What is created.** `createBot` (instructions become the bot's description), `importAgentMemory` (one line each in the
  bot's own `MEMORY.md`, up to 400), and `installSkill` for each skill (global, like any custom skill).
- **Limits.** 300 files, 256 KB each, 4 MB in all; symbolic links are not followed.

## Other clients

The terminal, Android and iPhone do not import: an agent's folder lives on the computer (under `~/.openclaw` or `~/.hermes`),
which a phone cannot browse, and the terminal has no folder picker. They see the bot that results.
