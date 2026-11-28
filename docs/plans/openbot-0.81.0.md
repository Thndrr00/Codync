# OpenBot 0.81.0 in Codync

Source: tag `v0.81.0` (102 bullets) and the 5 fragments on `origin/main@57668ad9a`, checked against
the 235 commits already on this branch.

## State

No new portable work. This release republishes changes already integrated from earlier fragments:
canvas video/audio assets, "Load remote content in chats", Bitwarden re-install, OMP instruction
reuse, public-header redaction, PDF save name, MCP sign-in with empty registration, plain image
decode errors, channel loading states, paused Marketplace apps, the own-MCP-address sign-in, and the
usage pace mark (`usage-pace.ts`, present since 0.72.0).

## Not portable (with reason)

- Joined servers and hosted account service: reconnect timing, D1 replicas, server-list polling,
  ownership requests, Signal update recovery, sync of agents/channels/pins, first-page message counts,
  background page loads, reaction/mark-read propagation, Apple Watch complications and logo.
- Chrome extension: hidden-server startup, connection screen, pasted images, reconnect, message sync.
- Web client: reconnect, silent-stop detection, web-channel messages, image sizing.
- OS packaging: Electron 45.3.4 for tiny web terminal text, Ubuntu/Docker glibc updates, TanStack
  Start XSS fix, self-hosted control socket and CLI hardening.
- Credits/prices: lower media prices, free-credit model rules, credit card motion.
- Website: plugin pages, plugin titles, comparison pages with Artificial Analysis scores.
- Behavioral telemetry/security of the account service and sign-out credentials.
- Performance-only internals: mailbox/SQLite cache, re-read reductions, UI menu/picker animation.

All remaining Codync-side items in `docs/plans/openbot-next-30.md` and the fragment batches are closed;
the only exclusions are the deliberate scope decisions above.
