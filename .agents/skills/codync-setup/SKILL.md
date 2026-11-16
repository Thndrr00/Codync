---
name: codync-setup
description: Build and run Codync on this machine (host, desktop, iPhone, Android, terminal) after cloning or switching branches. Use when a task needs a running app, a fresh build, or a check command for one platform.
---

# Codync setup

Resolve the repository root from the working directory and read its `AGENTS.md` first. Prefer the
commands in `docs/guides/development.md`; this skill only orders them and names the traps.

## Host

- `cd host && cargo build` builds the host. `cargo run -- serve` runs it in the foreground.
- CI checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- The host keeps its data in the data directory (the `data` folder next to the binary in dev, or
  `CODYNC_HOME`). Stop the old host before starting a new build: an old host speaks the old protocol.

## Desktop

- `cd apps/desktop && npm ci` (or `npm install --no-package-lock` when the lock is stale), then
  `npm run dev`. The renderer reloads on save; restart after `src/main` changes.
- After a host change in the main checkout run `node tools/native-host.mjs dev` first; `npm run dev`
  prefers that build over `host/target/debug`.
- Checks: `npm run typecheck && npm test`. Package only when an unpackaged run lacks something:
  `npm run dist:mac:dev` (about 45 s).

## iPhone

- `xcodegen generate --spec apps/project.yml` after editing `apps/project.yml`.
- Build with the `iOS Dev` scheme to `build/dd` (never another derived-data path). Keep one build.
- Package tests: `cd apps/ios/Kit && xcodebuild test -scheme CodyncKit-Package -destination 'platform=iOS Simulator,name=<iPhone>'`.

## Android

- JDK 21 and the Android SDK (SDK 37) are required. `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug`.
- Install Codync Dev with `adb install -r -d app/build/outputs/apk/debug/app-debug.apk`.

## Terminal UI

- The TUI is part of the host: `cargo run -- tui` (or the host's `--tui` flag). It talks to the same
  API as every client; no separate build.

## Before you start

- Every large feature or bug fix gets its own worktree and branch; stop the old app and host for the
  variant you rebuild.
- Sync first: `git pull --rebase --autostash origin dev` and push local commits that are not on
  `origin/dev`.
