# Block C validation — Windows, 2026-10-20

Behavior and native acceptance steps are documented in [Roster: search and
actions](roster-actions.md). The implementation covers ideas 17–24 in the host,
Android, desktop, iOS and terminal, using the existing Codync controls.

## Passing checks

- Host: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
- Host roster unit tests: six passed, covering Unicode search, visible message
  lanes, read-state preservation, filtering/order, folding and stale responses.
- `cargo test --test roster_e2e`: two passed against disposable real hosts,
  including pagination, a scope larger than 100 IDs, thread context, cross-bot
  entry rejection, main-chat preview and hidden-bot exclusion.
- Desktop: `npm.cmd run typecheck`, `npm.cmd run build`, and all nine tests in
  `roster-query.test.ts` / `roster-requests.test.ts`.
- `git diff --check`.

## General suite failures

The complete host binary unit suite finished with 311 passed, 24 failed and
one ignored. Failures concern agent process readiness, group/team agent fixtures
and market sign-in fixtures; several report readiness assertions or timeouts.
Those implementation and test files are outside this change. This run does not
establish whether they pass on another platform or a clean baseline.

The desktop suite finished with 149 passed, 11 failed and 19 skipped. Failures
are in the existing host-update/update tests; Windows path differences include
`\\Applications\\Codync.app` versus `/Applications/Codync.app`. Those source and
test files are outside this change. The general suites are not fully green.

## Checks still requiring native tools

- Android: the earlier roster query unit report contains three passing tests.
  The final cancellation, timeout and accessibility corrections received static
  review only. No Gradle task, APK compilation or APK installation was started
  after the user's instruction to defer it. Compose instrumentation and native
  acceptance remain pending. An earlier core run also had one cold-handshake
  timing failure; core code was not changed.
- iOS: source and call-site review completed; Swift compilation and Simulator
  checks require Xcode, which is unavailable on this Windows machine.
- Desktop native acceptance: Computer Use could not connect to its native pipe
  (OS error 2); browser preview automation was detached/unavailable. No visual
  or interactive verification is claimed. macOS/Linux native checks were not
  available in this environment.

The cleanup and read-only refactor reviews were completed. Applied findings
include retired search openings, explicit stop-confirmation errors, accessible
reorder guards, full project identity, preview ownership, terminal cursor/error
handling, large-roster search scope and visible-row clock subscriptions.
