# Agent accounts

An agent can keep more than one sign-in on a computer: the computer's own
(default) account and any number of isolated profiles. The host owns the
profiles and the sign-in; clients only list accounts, ask to add one, remove
one, or sign the default one out. Provider tokens stay inside the profile
folder on the computer.

## Isolated profiles

`host/src/agent/account.rs` keeps a profile per account under the host's data
directory (`accounts/<id>/account.json`) and points the agent at it with the
harness's own home variable:

| Agent | Variable | Credential file checked after sign-in |
| --- | --- | --- |
| Claude Code | `CLAUDE_CONFIG_DIR` | `.credentials.json` |
| Codex | `CODEX_HOME` | `auth.json` |
| Grok Build | `GROK_HOME` | `auth.json` |
| Gemini CLI | `GEMINI_CLI_HOME` | `.gemini/oauth_creds.json` |
| Google Antigravity | `GEMINI_HOME` (+ `AGY_ACP_FORCE_FILE_STORAGE=1`) | `antigravity-acp/acp_token.json` |
| OpenCode | `XDG_DATA_HOME` (+ `XDG_CONFIG_HOME=<profile>/config`) | `opencode/auth.json` |

A profile process also drops ambient provider keys (`ANTHROPIC_API_KEY`,
`OPENAI_API_KEY`, `XAI_API_KEY`, the Google/Gemini keys) so it cannot borrow
the computer's own sign-in.

Adding an account starts the agent with that profile environment:

1. If the agent advertises a browser/agent ACP sign-in method, the host calls
   `authenticate` on it.
2. Otherwise the harness's CLI login runs with the profile environment.
3. Antigravity keeps its `oauth-personal` settings and ACP flow.
4. The host checks the credential file above; a profile that did not produce
   one is removed instead of being saved.

## Removing and signing out

- **Remove account** deletes an isolated profile. The host first runs the
  harness's logout command inside that profile (best effort), then deletes the
  folder. A profile a bot still uses is refused until the bot picks another.
- **Sign out default** affects only the computer's own sign-in: the host runs
  the harness's logout command with the normal launch environment, or removes
  its credential files when it has no logout command (Gemini and
  Antigravity). Isolated profiles are untouched.

`accounts` and `signOut` on each backend in `hello` tell clients which agents
support each action. The desktop agents panel and the bot account picker on
desktop, iPhone and Android read the same host state. The TUI keeps its
existing Antigravity account selection and does not add or remove accounts.

Host methods: `listAccounts`, `accountAuthenticate`, `deleteAccount` and
`agentSignOut` (returns the refreshed `auth` check).

## Rotation

A provider may keep at most 20 isolated accounts. `account/rotation.rs` keeps one
stamped kv blob per provider (`account.rotation.<backend>`) with a mode —
`manual` (the default, so behaviour is unchanged), `roundRobin` (next account
each turn) or `onRateLimit` (change only once the active account runs out or is
rate limited) — a manually chosen account and a cursor. A turn reuses the same
profile environment path: a bot's pinned account always wins; otherwise the
policy picks an eligible account, skipping signed-out and resting ones through
the existing `fallback::rest` helpers, and advances the cursor only when it
changes. The host exposes `accountRotation`, `setAccountRotation` and
`accountStates` (state only — never tokens); the account sections on desktop,
iPhone and Android use the shared segmented control to set the mode, and the bot
account pickers show that a chosen account is pinned and ignores rotation. Setting
the mode reports `setting_changed` (`setting: account_rotation`, `value`: the mode),
never an account id or name ([analytics events](../reference/analytics-events.md)).

A rest a limit put one profile into is keyed `rest.<agent>:<account>`, so rotation can
skip that profile and keep the others eligible; out-of-credits and signed-out rests stay
on the provider-wide `rest.<agent>` key that older hosts wrote. The old key remains
readable after the change.

## Gemini's own tools and permissions

From OpenBot 0.77.0: a Gemini agent with Full access uses Codync's own tools — memory,
browser, channels and messages to other agents — without an approval card, the same as a
Claude agent. Shell commands, file changes and tools of other MCP servers still ask when
automatic approval is off. A Gemini agent confined to its workspace keeps sending messages
and using those tools without a card, but a command line, a file change or a path outside
the workspace still asks; the sandbox remains the authority on file writes. Other backends
keep their existing behaviour.

`host/src/agent/approval_rules.rs` holds the three pure decisions (`own_tool_allowed`,
`workspace_only_tool_allowed`, `tool_search_visible`) and recognises a call of one of
Codync's own built-in MCP servers by its ACP title or, for Antigravity, by
`ServerName`/`ToolName` in the request input. `on_permission` calls the
decision only after the ordinary rule check has refused forbidden and rule-denied requests,
and `SecurityProfile::permits_tool` keeps a profile's denial in force. `pi_tools.rs` keeps
tool search in the declared MCP tools when there are many of them or Computer Use is on,
so tools such as browser file upload stay discoverable.

From OpenBot nightly (`triage-13749`, `triage-13877`): a failed usage reading — a busy
provider or a timed-out read — does not sign an account out. It keeps its connection and
its last reading, so it stays available to rotation and backups; only a real sign-in error
signs it out. When Google asks to verify the account in use, or a provider refuses its
sign-in, and the pool has another ready account, that account is passed over for a short
rest scoped to it and the message is retried on the next ready account instead of failing
on the same one each time. With a single account there is nowhere to retry, so the older
provider-wide sign-out behaviour stands.

## Provider probes, processor support and sign-in pages

Three fixes from OpenBot nightly, all decided in the host so every client shows the same
state:

- **Grok model checks (`triage-15353`).** A Grok `session/new` is saved under
  `~/.grok/sessions/<encoded cwd>` and has no `session/delete`. Model discovery uses a probe
  folder (never a bot's chat session) and removes the probe's session file after the read,
  so a repeated model check no longer fills the Grok session list. The host only removes a
  file whose id and encoded cwd match the one it just opened.
- **Unsupported processor (`triage-15358`).** Gemini and Antigravity CLI builds need SSE4.1;
  a build without it dies with SIGILL before it can print its own explanation. The host
  classifies that one signal as *"The processor of this computer cannot run this provider.
  Use a different provider."*, shows it on the Providers row, and keeps the original error
  in the log. A download, a network error or a crash while a bot runs is never shown as a
  processor problem.
- **Claude's sign-in page (`triage-15344`).** When a CLI's own opener fails it still prints
  the page, so Add account now reads that link and opens it from the app on every OS, the
  same page-open path ACP agents use when they post one. Only a plain https link on the
  provider's own domain is opened; a link carrying a code or token is ignored. The desktop
  "Sign in" (Connect) already opens links printed in its setup terminal the same way.
