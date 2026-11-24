# Bot import

People coming from Grok Bot (or from an app that already imported their Grok Bot agents) bring their agents
over in one step: an export file (`.zip`) on this computer becomes bots and group chats with their
instructions, avatars, skills, routines, memories and workspace files. Chats aren't copied; the export
keeps what matters in them as memories.

## Flow

1. The user adds the export skill to an agent in Grok Bot. The skill is `host/src/import/export-skill.md`
   (`EXPORT_SKILL`); the desktop app copies it (**+ → Import bots… → Copy export skill**, host method
   `importSkill`) and `codync-host import --skill` prints it.
2. They ask that agent "Export my agents for Codync". It writes `~/Downloads/codync-export-<date>.zip`.
3. They choose the file: desktop **+ → Import bots…** (`app:pickExport` opens the dialog in the main
   process and returns a path), or `codync-host import <file>` (`--dry-run` lists it, `--only a,b` picks
   agents). Both call `previewImport` then `importBots` with the path; both methods are loopback only
   (`LOCAL_ONLY`), since the path is on this computer.

Phones and the terminal UI have no import screen: the export file is on the computer, and the CLI covers
the terminal.

## Formats (`host/src/import/manifest.rs`)

| Manifest | `format` | Written by |
| --- | --- | --- |
| `codync-import.json` | `codync-bot-import` | Codync's export skill |
| `openbot-import.json` | `openbot-agent-import` | Other apps' Grok Bot export agents |
| `openbot-transfer.json` | `openbot-instance-transfer` | Those apps' own transfer files (agents, memories, sections) |

The three share one shape (agents with `key`, `name`, `title`, `description`, `avatar`, `skills`,
`routines`, `memories`, `files`; optional `channels`). `version` above 1 is refused. A manifest inside one
top-level folder is found too. An agent, group or routine that doesn't check out is left out with a
warning; the rest imports.

## What each part becomes (`host/src/import/mod.rs`)

- **Bot**: `createBot` with defaults; `description` = title, then instructions. A transfer file's
  `provider` is kept when that agent runs on this computer, else the bot gets the default agent and a
  warning; its model isn't (other apps' model ids aren't ACP ids).
  Colors and shapes cycle through the avatar choices.
- **Avatar**: `setIcon` for `bot:<id>`; needs the computer signed in, else a warning and the default face.
- **Skills**: each folder becomes a skill of that bot's own (`add_custom_skill` written by the bot, on for
  it only), with the folder's other files beside `SKILL.md`.
- **Routines**: schedules map to cron, interval or one-time triggers (`schedule.rs`) in the routine's time
  zone (else this computer's).
- **Memories**: into the bot's `MEMORY.md` as imported facts, through the same path as the old memory move
  (`chat::memory::add_facts`): checked, capped, the rest into `earlier.md`. The count reported is what was
  written.
- **Workspace files**: copied to `imported/` in the bot's workspace.
- **Group chats**: a group of the imported members; its routines and memories go to its lead (or first
  member), since Codync keeps them per bot. A group whose bots already share one (two channels of the same
  members, or one the user made) isn't made again: a warning says it joins that group.

A bot whose skills or files fail is deleted again with the skills and `imported/` files it got; the others
carry on. Import counts as `bots_imported` in analytics (source and counts only).

## Duplicating an agent

**Duplicate** on a bot copies its setup into a new bot: same agent, skills and persona, but no chat,
pinned place or owner. A bot with its own workspace also gets a copy of that folder before its actor
starts, and its config points at the copy. The copy keeps linked folders (a Windows junction for
installed packages, for example) as links instead of walking into them, so it cannot fail on a
package cache. When Windows security or search briefly holds a newly created file, the copy waits a
short, capped backoff and tries that one step again; any other error fails the duplicate, which
leaves no half-made bot behind. Bots that point at a shared project folder keep that folder as
before, and nothing is copied for them. The pure retry decision and its tests are in
`host/src/store/bots/workspace_copy.rs`.

## Safety (`host/src/import/archive.rs`)

Entries are listed before anything is read: at most 5,000 entries and 500 MB inflated. Paths that leave the
archive and links are skipped; so are `.env*`, `*private*key*`, nested archives, `.git` and `node_modules`
(reported back), and Mac zip noise (silently). Every manifest path of an agent must sit in
`agents/<key>/`, and keys are lowercase letters, digits and hyphens.
