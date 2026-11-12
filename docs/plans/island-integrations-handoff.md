# Island project and workflow integrations

Worktree: `C:/Users/Edwin Villeda/Documents/Codync-island-integrations`.
Branch: `feat/island-integrations`, based on `e81ac7de` from the previous
island workspace batch. This branch includes all preceding island/usage
commits. The shared checkout was untouched. `git ls-remote --heads origin dev`
returned no branch, so a shared `origin/dev` pull was unavailable. No merge or
push is authorized; this is a local commit for another agent to integrate.

## The fifteen integrations

1. GitHub same-origin-branch PR state and requested/submitted reviews.
2. Recent repository issues assigned to the host's connected GitHub account,
   showing their current state and assignees. No invented per-bot issue mapping.
3. PR head-commit checks and classic statuses, with bounded reads and partial
   errors. Unavailable checks never become a clean/passed result.
4. Existing agent setup terminal status/exit codes and exact-session reopening.
   Focus a still-live original owner instead of navigating away and killing its
   process. Other retained sessions attach without starting or owning a process.
5. Targeted remote computer stream reconnect, with failure feedback.
6. Upcoming routine occurrences from the existing host flow-canvas preview.
7. Host-classified low-confidence memories linked to the existing memory panel.
   These are beliefs to review, not a new formal review-request queue.
8. Connector sign-out/failure states linked to its installed sign-in controls.
9. Artifact revision notices from existing SSE metadata/library refresh, linked
   to the latest revision in the existing viewer, not historical-version access.
10. Current synchronous delegated requests from the host's authoritative wait
    registry, scoped to the selected bot and without message bodies.
11. Pending approval age using the real entry creation timestamp.
12. Upload/sending state from existing attachment IDs and progress stores;
    missing progress is unknown/sending rather than invented zero/completion.
13. Main-window/island dictation metadata and guarded stop controls, without
    acquiring a microphone, copying transcripts or changing call ownership.
14. Main-lane scratchpad capture using existing bounded/mutex-protected storage;
    running routine/group lanes cannot redirect the append.
15. Saved searches scoped to the selected bot and bookmarks opening the existing
    host-validated context viewer with exact entry/thread IDs.

## Ownership and compatibility

The host owns GitHub I/O, repository association, terminal processes, pending
bot waits, scheduling, memory classification and scratchpad writes. Desktop
stores project snapshots and device-only state, while views render and send
intents. Tokens, terminal output, memory version bodies, full conversations and
audio are excluded from summaries. Explicit saved-search results contain only
bounded previews; quick-note text is sent only after an explicit save.

New methods `islandGitHub`, `listAgentTerminals`, `islandDelegations` and
`appendBotScratchpad` require Local or authenticated Control access. Screen-only
and pairing callers are rejected. Control access supports remote bot selection;
it is intentionally not loopback-only. Existing methods/fields are not removed
or renamed, so compatibility minimums are unchanged. Older hosts show partial
unsupported-resource errors while supported sections remain usable.

GitHub reads use the existing vault connection and hardened egress client.
Only strict github.com origins are accepted; requests go to api.github.com.
Same-origin relocations are rejected after the egress client handles them;
cross-origin redirects are already refused. Bodies are capped at 1 MiB, the
HTTP snapshot budget is 15 seconds, and a maximum-16-entry cache lives for
60 seconds. A named branch and configured token are required. Fork-base PR
association, GitHub Enterprise and historical issue/assignment-event timelines
are not claimed. A token/account generation change during HTTP invalidates the
response before it is cached/returned.

The integration refresh interval is one minute while enabled; connection,
dictation, uploads and known artifact revision signals follow existing device
events. Unknown/future approval timestamps remain unknown. Scratchpad capture
is limited to 4,000 Unicode characters per intent and the existing 16,000 total.
Only successful matching acknowledgements clear unchanged submitted text;
accepted saves remain acknowledged after switching away and back. Resource and
search actions are scoped to exact known snapshots and reject stale owners.

Inspected corresponding iOS memory/retry/artifact/composer surfaces, Android
store/library surfaces and terminal memory/library surfaces. This change adds
shortcuts and status presentation inside the desktop OS overlay; those clients
have no Electron floating panel or window-local setup/dictation ownership.
Their existing screens and system Live Activity containers retain their
behavior. The shared host APIs are additive and reusable. Desktop macOS,
Windows and X11 Linux share the implementation; Wayland remains unsupported.

Version 2.44.2 is inherited and already ahead of the fetched v2.44.1 tag, so no
second bump is made. Reconcile the release version once during integration.
The earlier usage-fix handoff's XcodeGen regeneration remains pending on macOS.

## Review and validation boundary

The user permits code and code tests only: no builds, native compilation,
packaging, app launches, visual tests, E2E, simulators or Computer Use. Native
tests are authored but not executed. No app/host process or user Computer Use
configuration was changed. Native compilation and actual UI behavior remain
unverified and must be checked by the integrating agent.

Correctness, completeness and cleanup reviews ran read-only, followed by a
verifier. Confirmed fixes include partial GitHub visibility, global connection
errors, a single width-appropriate memory surface, targeted/repeatable connector
navigation, GitHub in-flight account invalidation, UTF-8 copy, revision-notice
retention across failed reads and safe scratchpad acknowledgements. A focused
review also confirmed and fixed original terminal-owner destruction on reopen.
The claimed missing EmptyState import was refuted: the extracted InstalledView
defines it locally. No unrelated cleanup expansion was needed.

Final code checks:

- Full desktop Node suite with `--test-concurrency=4`: 668 total, 644 passed,
  24 skipped, zero failures. This runs the same file globs as `npm test` with
  lower file concurrency. The batch adds 38 Node regression tests.
- `npm run typecheck` (`tsc --noEmit`): passed, with no application binaries.
- Rust formatting on all changed Rust files, strict UTF-8 reads of changed
  text files, and `git diff --check`: passed.
- Eleven new Rust regression tests are authored but not executed. They cover
  strict GitHub origin parsing, metadata projection, account replacement, setup
  terminal metadata, delegated wait snapshots and scratchpad lane/bounds rules.
  The existing API permission test also covers the new methods.

An earlier full default-concurrency run passed at 660 total tests. The final
default-concurrency attempt had one `ECONNRESET` in the unchanged SSH listener
ownership test (`shared/ssh-platform.test.ts`), with 643 passed and 24 skipped.
Its isolated rerun passed all six SSH platform tests; the complete final run
with concurrency four passed as above. No cause was established and no SSH
source/test was changed. This records an intermittent validation result,
not a proven preexisting defect or an unconditional default-run pass.

## Fifteen further proposals

Not implemented in this batch:

1. Active context-window use and compaction notices.
2. Per-bot token and cost counters from host usage reports.
3. Token-budget warnings linked to existing bot budget settings.
4. Agent fallback changes and the host's reason for switching.
5. App/host update download and installation progress.
6. Selected computer CPU, memory and disk pressure details.
7. Recently edited documents linked to the document viewer.
8. Favorite bot mini-apps opened from the island.
9. Active browser/computer-use session status and open-session action.
10. Current MCP tool state linked to its existing trace.
11. SSH tunnel connection state and recovery action.
12. Channel delivery failures linked to existing channel diagnostics.
13. Git conflict files linked to the project conversation.
14. Active worktree identity and navigation between project worktrees.
15. Pending device pairing/access requests linked to existing approval flows.
