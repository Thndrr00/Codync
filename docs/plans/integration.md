# Integration branch: `integrate/next-wave`

This branch gathers the next-wave work so it can be reviewed and compiled in one place. It is built
from `feat/next-wave-tools` and merges, in order:

1. `feat/openbot-connectors` — the 249 OpenBot connectors, the `--apps-only` importer and the market
   check tool.
2. `feat/openbot-routine-canvas` — the routines page and canvas.
3. `feat/comfy-bundled` — bundled ComfyUI, its tools window and MCP routes.
4. `feat/hermes-integration` — Hermes runtime card, local OpenAI endpoints, native channels and the
   skill history/restore tools.

Conflict resolutions kept both sides:

- `AppRail`: one rail with Inbox (with its count), Board, Routines, bot canvas, Usage, n8n, Browser,
  ComfyUI and Extensions.
- `ChatWindow`: the Inbox/Board/Routines pages share the compact rail and each clears the others; the
  ComfyUI tool state reaches the rail; the browser and ComfyUI buttons coexist.
- `store/mod.rs`: both `notification_rules` and `native_channels` are declared.
- `api/bots.rs`: the `chatCall` keeps the tool policy, cache and limits, and uses
  `skills::dispatch` so history/restore work.
- Telegram and Slack keep the channel-notes wrapper (`with_notes`), while Telegram takes the Hermes
  route and live-card flow through `telegram::input`.
- The Spanish strings file keeps both new key sets.

Nothing here has been compiled or run. The next step is a build, `cargo fmt --check`, Clippy,
`cargo test`, desktop `typecheck` and `test`, then the Android and iOS checks.
