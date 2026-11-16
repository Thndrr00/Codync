# Computer containers

A bot's computer can run **on this host** (as Codync always has) or in **its own Docker container**, chosen per bot.
The agent loop doesn't change: the workspace tools read the bot's folder on the host, and the shell tool runs the same
command here or through `docker exec` in the container.

## What a container gets

- Its own copy of the bot's workspace, mounted at `/workspace` (the host folder is the same one the Files tab reads, so
  the workspace survives a container being replaced).
- Its own named home volume (`codync-bot-<id>_home` at `/home/codync`): tools, shell history and installed packages
  stay with the bot.
- Its own browser profile folder under the host's data directory, mounted at `/profile`; it is not inside a project
  workspace.
- The image `codync/computer:1` unless the bot's settings name another. Build it once with
  `docker build -t codync/computer:1 packaging/computer`; it carries a shell, git, curl, jq, ripgrep, Python and
  Node. Any other image must provide `sh` and write to `/workspace`.

## Isolation, and what it is not

The container is made with `--cap-drop ALL`, `--security-opt no-new-privileges`, `--pids-limit 512`, `--memory 4g`
and `--restart unless-stopped`. The Docker socket is **never** mounted, no host path other than the workspace and the
browser profile is mounted, and no host environment variable is passed into `exec`. If Docker lists gVisor
(`runsc`), the container runs with `--runtime=runsc`; otherwise it uses Docker's default runtime and the panel says
so.

This is the best Codync can do on Docker, not a micro-VM: the Docker engine and its kernel are shared with the host.
It isolates a bot's files and tools from another bot, not from a Docker escape. The browser still uses the host's
browser driver; the container provides the profile folder, not a separate browser process. Published ports are never
opened by this feature, so nothing in the container listens on a host address.

## When Docker is missing

A container bot checks Docker before each shell command. If Docker isn't installed or isn't running, it runs the
command in its local workspace instead, and the computer panel says why. `fallbackLocal` (on by default) controls
this; turn it off to make the shell fail instead.

## API and settings

- `computerPanel {botId}` now answers with `runtime` (what runs now, Docker engine, gVisor, container name, image,
  running, detail), `settings` (the saved choice) and `browser` (profile path).
- The bot's `computerRuntime` setting: `{kind: "local" | "container", image, gvisor, fallbackLocal}`. It is saved with
  `updateBot`, like every other bot setting.
- `runComputerCommand` (and the agent's `workspace_shell`) goes through the container when it is running; Files stays
  a host read of the mounted folder.

## Screens

| Client | Where |
|---|---|
| Desktop | Computer → Status: **Local / Container**, the runtime detail, image, gVisor and browser profile |
| Android | Computer → Status: the same choice and detail |
| iPhone | Details → Computer: **Local / Container** under "Use the computer" |
| Terminal | Computer → Status: `w` switches local/container; the runtime line says what runs |

The fallback is visible in every panel: a container bot without Docker says it is using the local workspace.

## Validation

`host/src/computer/runtime.rs` tests cover the default choice, container naming, image validation, gVisor discovery,
the Docker arguments (workspace, profile, no socket, no privileged mode, no host network) and the `docker exec`
command. The host was not built or run in this change; Docker itself was not exercised.
