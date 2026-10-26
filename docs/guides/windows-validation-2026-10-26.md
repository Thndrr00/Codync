# Windows integration validation — 2026-10-26

The local `integrate/windows-all-additions` branch combines the catalog/team
branch, Mercury, snapshots of the uncommitted Openstark and Bdots work, and
Android block E. Original branches and worktrees were preserved. `origin/dev`
does not exist in this fork; the comparison base is `origin/main` at eded5f5f.
Historical Windows/Android integration changes already present in main were
not replayed over newer fixes. Its remaining Android analytics work is not a
Windows addition and was not ported in this validation.

## Installed application

- Codync Dev 2.26.0, Windows x64, installed with the generated NSIS installer.
- Installed host SHA256 matches the compiled native host.
- Normal per-user host service runs on port 19223 with the existing Dev data.
- Production on port 19222 was left running and unchanged.
- Disposable API/UI tests used a separate data directory and port 19480.

## Corrections

- Shell jobs resume their Windows process and drain both output streams while
  waiting, including output beyond pipe capacity.
- Bot imports validate first and roll back partial creation with visible errors.
- Bot creation retains an explicitly selected unavailable agent.
- Sentinel views use the actual host schema, dates, issue severity and evidence.
  Android uses the same schema; iOS/TUI have no equivalent Sentinel screen.
- Interval schedules are connected to the current routine editor and preview.
- Removed a redundant session-lifetime MCP retry guard that blocked legitimate
  repeated calls; the authoritative per-turn/stall protections remain.
- Resolved integration compilation mismatches and Clippy findings.
- Windows ACP tests resolve a real Python executable. The memory-file test agent
  no longer executes historical commands quoted in background memory reviews.

## Completed checks

- Desktop: TypeScript and 329 tests pass; 19 platform-specific tests skipped.
- Host: formatting, workspace/all-target Clippy with warnings denied, 633 host
  unit tests, supervisor tests and 47 memory-crate tests pass.
- Host integration suites pass, including catalog rollback, imports, ACP process
  cleanup, files, memory, memory files, teams, turns, routines and CLI delivery.
  The wire-shape golden contains seven additive proactive-setting paths.
- Node host SQL/security tests: 44 pass.
- Installed-host API checks: 17 pass, covering teams, bundles, saved searches,
  schedules, miniapps, WebSocket updates, view-link revocation and shell jobs.
- Installed renderer: local onboarding, explicit Amp selection, catalog,
  search modes/scopes, empty search, agent startup failure, diagnostics with a
  real issue and rescan, and saving a two-hour interval routine were exercised.
- Final normal Dev profile opens with its existing bot and a healthy local host.
- Android: 217 core tests (one skipped), 67 app tests, Kotlin compilation and
  debug APK assembly pass. Lint did not complete; its live thread dump remained
  in `JoinEffectDetector` fixed-point analysis. It is not counted as passed.

## Limits and follow-up

The native Computer Use pipe was unavailable (OS error 2). UI checks used CDP
against the installed Electron renderer, with screenshots under
`build/windows-ui`. Microphone, external agent authentication, real remote
computers, iOS/macOS/Linux UI and Android device UI were not exercised here.
Some added labels remain in English in the Spanish interface.

Android's existing lock-screen summary receiver still renders the ordinary bot
widget. This needs separate device validation and a focused privacy/UI fix;
the Windows application does not use that receiver.
