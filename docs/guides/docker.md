# Running the host in Docker

`codync-host` is one Rust binary that runs your bots and serves the Codync apps. This page
covers the image in the repository root (`Dockerfile`, `docker-compose.yml`, `.dockerignore`):
why you would run it, how, and what a container does not change.

The short version: it is the same host you would install on a Linux server
([Linux servers](linux-servers.md)), with the data directory on a volume instead of
`~/.codync`. It is not a workspace platform: one container runs every bot, exactly like one
computer does.

## When it helps

- A server you already run containers on, where you want the host beside the rest of your stack.
- A pinned, reproducible host version: the image is the version.
- Trying the host without installing anything on the machine.

## Requirements

| Need | Why |
| --- | --- |
| Docker with Compose v2, or just Docker for `docker run` | The compose file uses the `name:` key and a healthy `depends_on` |
| Linux host for `network_mode: host` | Only Linux supports it; see [Pairing](#pairing) |
| An agent inside the image | The image ships the host only, no Node.js, no `claude`/`codex`: without one no bot can answer (see [What Docker does not give you](#what-docker-does-not-give-you)) |

## Build

From the repository root:

```sh
docker build -t codync-host:local .
```

The build compiles the host from `host/` in a Rust stage and copies only the binary into a
`debian:bookworm-slim` runtime. `CODYNC_ENV=main` is compiled in, so the image is a production
host: port 19222, `~/.codync` (here `/data`), `https://api.codync.dev`. A dev build would be
port 19223 and the dev cloud, which a server image must not reach.

On an Apple Silicon or arm64 machine, build for the architecture the server runs:

```sh
docker buildx build --platform linux/arm64 -t codync-host:local --load .
```

`--platform` builds the whole Rust stage under emulation when the host is another
architecture: correct but slow (tens of minutes). Building on the target architecture, or on
your CI's native runner, is the fast path. Override `--build-arg RUST_VERSION=…` only if the
pinned toolchain in `host/rust-toolchain.toml` moves.

## Run

```sh
docker run -d --name codync \
  -p 19222:19222 \
  -v codync-data:/data \
  --restart unless-stopped \
  codync-host:local
```

Check it, then follow its log:

```sh
curl -s http://127.0.0.1:19222/health
docker inspect --format '{{.State.Health.Status}}' codync
docker logs -f codync
```

`/health` answers `{"ok":true,"hostId":…,"computerId":…,"version":…,"environment":"main"}`
without a token. The image's `HEALTHCHECK` calls the same route through
`/usr/local/bin/codync-healthcheck` (source: `docker/healthcheck.sh`), so
`docker run` and Compose report the container unhealthy when the host stops answering.

The container runs as the unprivileged user `codync` (uid 10001) with `HOME=/home/codync`
and `CODYNC_HOME=/data`. The entrypoint (`docker/entrypoint.sh`) runs
`codync-host serve --bind 0.0.0.0 --port 19222`; override the port with `-e CODYNC_PORT` and
the bind with `-e CODYNC_BIND`. Any other command passes through, which is how
`docker exec` gets the CLI:

```sh
docker exec -it codync codync-host status
docker exec -it codync codync-host doctor
docker exec -it codync codync-host tui
```

`docker run` and Compose are alternatives: pick one. The volume here is named `codync-data`;
Compose names its own `codync_codync-data`, so adjust the backup command below if you use this
form.

## Compose

```sh
docker compose up -d --build
docker compose logs -f codync
docker compose exec codync codync-host status
```

`docker-compose.yml` has one service, `codync`, the named volume `codync-data`, and the
environment the host needs: `CODYNC_HOME=/data`, `CODYNC_BIND=0.0.0.0`, `CODYNC_PORT` (default
19222, used for both sides of the port mapping) and `CODYNC_EXTERNAL_URL` (empty unless you run
the proxy). Put values in a `.env` file beside the compose file, or export them.

The profile `proxy` adds Caddy with TLS in front of the host, forwarding **only** `/channel`:

```sh
CODYNC_DOMAIN=codync.example.com CODYNC_EXTERNAL_URL=https://codync.example.com \
  docker compose --profile proxy up -d
```

There are no profiles for the WhatsApp bridge or n8n because they are not separate processes
in Codync: the host starts both itself, as children, and talks to them over a pipe and
`127.0.0.1`. In a container that means they run inside the host's container, and they need
Node.js and their packages there — neither is in this image. See
[Extending the image](#extending-the-image).

With the proxy in front, keep the host's own port off the network: bind it to loopback
(`127.0.0.1:19222:19222`) or drop the mapping, so TLS is the only way in.

## Data, backups and upgrades

Everything the host owns is under `/data`: the SQLite database, the loopback token, the
device identity keys, the credential vault's id, bot workspaces (`/data/bots/<id>/workspace`),
WhatsApp media (`/data/whatsapp-media`) and, if you use n8n, its program
(`/data/n8n-program`, several hundred MB). Copy that volume to back the host up:

```sh
docker compose stop codync
docker run --rm -v codync_codync-data:/data -v "$PWD":/backup alpine \
  tar czf /backup/codync-data.tgz -C /data .
docker compose start codync
```

Restore is the same command with `xzf` into an empty volume. With a bind mount instead of a
named volume, the copied files must belong to uid 10001:

```sh
sudo chown -R 10001:10001 ./codync-data
```

To upgrade, take the new files and rebuild; the volume keeps its data:

```sh
git pull
docker compose up -d --build
```

This fork's builds take no release updates ([updates](updates.md)), so the image is the update
unit either way. Never run a self-update inside the container, even on a build that follows
releases: it would replace the binary in a layer you are about to throw away, and a restart
would undo it. `docker compose down` keeps the volumes; `docker compose down -v` deletes them.

## The port, the token and loopback

The host listens on `0.0.0.0:19222` inside the container, and the port mapping publishes it.
What answers depends on the route, and it is the same rule as on any other computer:

| Route | Who gets in |
| --- | --- |
| `GET /health` | Anyone who can reach the port: host id, computer id, version, environment. The binary path and hash, `busy` and `updateRequested` go to loopback callers only |
| `POST /api/<method>`, `GET /events`, `/term`, `/oauth` | Loopback **and** `Authorization: Bearer <token>`. The token is `/data/token` on the volume: `docker compose exec codync cat /data/token`. Keep it there; it lets a loopback caller do anything the apps do |
| `GET /channel` | Phones holding this host's pairing keys, end to end encrypted. An upgrade without `?v=1` answers `426`, a plain request `400`: that is what a working proxy looks like |
| `/hooks/routines/{id}`, `/hooks/slack`, `/hooks/teams/{id}` | The routine's key, Slack's signature or Microsoft's JWT |

So publishing 19222 exposes `/health` and `/channel` to the network, not the API. Keep the port
closed in the firewall anyway, exactly as on a Linux server, unless you deliberately pair over
a VPN or a private network.

If you put a reverse proxy in front, forward only `/channel` and set `CODYNC_EXTERNAL_URL` to
the URL it serves (the compose `proxy` profile does both). A proxy on the same machine reaches
the host at `127.0.0.1`, so every request it forwards looks loopback and the loopback-only
checks stop protecting `/api`, `/events`, `/term` and `/oauth`: forwarding them would publish
your host to anyone who reaches the proxy. The compose proxy is a container of its own, so the
host sees its address rather than a loopback one — but the rule is the same, because relying on
that difference is not a security boundary. The warning and its proof are in
[Linux servers](linux-servers.md#reaching-it-from-the-phone).

## Pairing

With bridge networking (the default, and the only one on macOS and Windows), the host sees the
container's address — `172.x.x.x` — not the machine's LAN or Tailscale address, so the pairing
link advertises an address your phone cannot reach. Three ways out, in order of least setup:

- **The Codync relay.** Put the host in your Codync account and sign the phone in to the same
  account: the container connects out, so pairing and later connections work through the relay
  with nothing published.
- **Host networking on Linux**, so the host sees the machine's real interfaces:
  `network_mode: host` in the compose service (delete `ports:`; the host binds 19222 on the
  machine itself). Then Tailscale or the LAN address in the pairing link works.
- **Your own HTTPS reverse proxy** with `CODYNC_EXTERNAL_URL` set, as above.

Either way the pairing starts on the computer:

```sh
docker compose exec codync codync-host pair          # QR code, then it asks about the phone here
docker compose exec codync codync-host pair --json   # the link only, for a script
```

The code works once and expires after ten minutes. If nobody is at a terminal to answer the
computer's question, `codync-host access approve <code>` does it from a script (add
`--phone-only` to keep the host out of the account). What the phone shows and which answer
lets it in are the same as for any server ([Linux servers](linux-servers.md#reaching-it-from-the-phone)).

## Extending the image

The host is the only thing installed: the image starts, answers `/health` and runs its
commands, but no bot can answer a turn until its agent is there. Save this as
`Dockerfile.tools` and build it with the repository root as the context
(`docker build -f Dockerfile.tools -t codync-host:tools .`):

```dockerfile
FROM node:24-bookworm-slim AS node

FROM codync-host:local
USER root
# git for a bot's commits; Node.js for the npx-based agents and the WhatsApp bridge, and the
# oldest major n8n runs on (host/src/n8n_program.rs).
RUN apt-get update \
    && apt-get install -y --no-install-recommends git \
    && rm -rf /var/lib/apt/lists/*
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules/npm /usr/local/lib/node_modules/npm
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx
# The WhatsApp bridge the host starts itself: `host/src/whatsapp.rs` looks for it beside the
# host binary, so it goes there and brings its own modules.
COPY host/whatsapp-bridge /usr/local/bin/whatsapp-bridge
RUN cd /usr/local/bin/whatsapp-bridge && npm ci --omit=dev
USER codync
```

Then sign the agents in inside the running container as `codync`
(`docker exec -it -u codync codync claude`), or copy an existing `~/.claude` and `~/.codex`
into `/home/codync`. The session files an agent keeps in `HOME` live in the container, so keep
`/home/codync` on a volume too if you sign in once and want it to survive a rebuild.

## What Docker does not give you

- **No per-bot container.** Every bot shares the host process, the data directory and the
  agent CLIs, as on a laptop. Bots are separate conversations, not separate kernels.
- **No gVisor, Kata or Firecracker.** The container boundary is the container's own, not a
  per-turn sandbox; Codync's approval cards and per-bot permissions are what constrain a bot,
  not a hypervisor.
- **No web client.** Codync's clients are the iPhone and Android apps, the desktop app and the
  terminal UI. The host serves no browser UI, in a container or anywhere else.
- **No credential vault without a Secret Service.** On Linux the vault is the desktop's secret
  service (D-Bus). A container has none, so connectors, Composio and stored API keys are
  unavailable and new bots start without connectors, the same as on a headless server. There
  is no supported file-based vault in a production build.
- **No computer use or remote screen.** They need a desktop session, and the image is Linux
  without one.
- **No systemd.** `codync-host install`, `stop` and `uninstall` manage a launchd/systemd
  service; in a container the container manager plays that role. Don't run them.
- **No auto-updates.** The image is the version; see above.

## Building in CI

The repository has no Docker job (`.github/workflows/host.yml` builds the release archives), so
this change adds none. A job that wants one needs only:

```sh
docker build -t codync-host:ci .
docker run -d --name codync-ci -p 19222:19222 codync-host:ci
until docker exec codync-ci /usr/local/bin/codync-healthcheck; do sleep 2; done
```

The health check script is the whole verification and exits non-zero when `/health` does not
answer. Add `--platform linux/amd64,linux/arm64` with `buildx` if you publish both
architectures; without `--load` or `--push`, `buildx` builds without keeping the image.

## Verification

Not yet built or run: these files arrived on 2026-11-15, and no image was built, started or
paired in that change. A first pass should confirm: the image builds for both architectures,
`docker compose up -d` reaches `healthy`, `/health` answers with `"environment":"main"`, the
volume keeps the token and bots across `docker compose down && docker compose up -d`,
`docker compose exec codync codync-host doctor` reports the setup with no bot, and a phone
pairs over the route you chose above.
