---
name: codync-release
description: Ship a Codync change: version bump, What's New, release gates, cloud deploy and the compatibility check. Use when a change to shipped code is ready to merge or when asked how Codync releases.
---

# Releasing a Codync change

Read `docs/guides/development.md` and `docs/guides/environments-and-deployment.md` first; this skill is
the checklist, not a replacement for them.

## 1. Version

- Every source change under `apps/`, `host/` or `packaging/` bumps the version before it reaches
  `main`: `MARKETING_VERSION` in `apps/project.yml`, `version` in `host/Cargo.toml` (and `Cargo.lock`),
  and `version` in `apps/desktop/package.json` (and its lock), all the same value.
- Patch for fixes and small tweaks, minor for features or protocol additions. Never bump major
  (stay on 2.x). One bump per merge into `main`; if the version is already ahead of the latest `v*`
  tag, leave it.
- A change to `host/crates/memory` also raises `codync-mem`'s 0.x version once per release (CI blocks
  a PR that forgets it).
- Run `xcodegen generate --spec apps/project.yml` after the bump. Never touch `CURRENT_PROJECT_VERSION`.

## 2. What's New

- Write the release's section in `apps/ios/WhatsNew.md` (App Store) and `apps/android/WhatsNew.md`
  (Google Play) when a phone app changes: zh-Hant + en-US, in the words the app's users notice. Fill
  the PR template's *What's New* bullets too.
- The catalog test checks that every announcement string exists in every language catalog.

## 3. Gates

- Host: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
- Desktop: `npm run typecheck && npm test`. Android: the Gradle checks. iPhone: the package tests.
- End-to-end on the installed app for every changed flow; full e2e for platform, packaging or desktop
  main-process changes (macOS, Linux, Windows).
- Compatibility: a rename or removal raises `minApp` in the host's `hello` and `minHost` in the client
  in the same release; additive changes need nothing. The release gate holds until the App Store has
  the phone build ([compatibility](../../../docs/reference/compatibility.md)).

## 4. Cloud

- `cloud/` is deployed by hand, never by CI: run `npm run deploy:dev`, test the dev environment, merge,
  then `npm run deploy:main` (plus any new bucket or migration) and check the new routes.

## 5. Commit and merge

- Conventional Commits in English, one logical change per commit, the version bump as
  `build: bump version to X.Y.Z`.
- PRs into `main` explain behaviour, link issues, list validation and include screenshots for UI.
- After merging an external PR, leave a thank-you comment.
