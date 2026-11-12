# Island workspace integrations

Worktree: `C:/Users/Edwin Villeda/Documents/Codync-island-workspace`.
Branch: `feat/island-workspace-tools`, based on `a3ffcc82` from
`feat/island-control-center`. This branch includes the previous island controls
and cross-client dotted usage corrections. It is for local handoff only: no
merge or push. The fetched origin has no `dev` branch, so shared-checkout sync
against `origin/dev` was unavailable; the shared checkout was not changed.

## Implemented fifteen

1. Workspace search for bots, project paths and existing island actions.
2. Opt-in global shortcuts for toggle, search, activity, message, usage and
   quiet. Preserve requested settings and show registration errors; reserve
   existing application/approval shortcuts and unregister only owned keys.
3. Activity filters for all, selected bot, or exact project on the same computer.
4. Retry host-eligible routine results with the original `retryRoutineRun` ID
   and retained event, rather than starting a replacement routine.
5. Actual actor FIFO waiting-user-message list and acknowledged per-entry
   cancellation. Preserve the active turn and unrelated background work.
6. Read-only previews of recent completed edits, excluding proposed permissions,
   plus registered files opened with the existing viewer. Bound patch previews.
7. Recent artifact metadata linked to the existing conversation artifact viewer.
8. Host-owned Git branch and changed-file count with bounded read-only probes.
9. Resume a saved draft inside the island. Transfer its text only on explicit
   request, retain its original on close, and preserve concurrent main-window
   edits when sending. Reuse the normal recoverable send path.
10. Manual Quiet profile and optional quiet while another Codync window is
    fullscreen. Preserve approvals and manual interaction. External apps require
    manual quiet; this does not claim OS-wide fullscreen detection.
11. Independent island text size (11–18) and high contrast, including shared
    control scaling without changing the main app's text preference.
12. Host-counted trailing 24-hour summary, labelled Last 24 hours. This is not
    a calendar-day count and identifies the selected computer.
13. Persistent local short notice/error history, clearable, capped at 50 rows.
14. Repeated notice grouping with occurrence counts, not poll counts. Clearing
    acknowledges still-active notices so they do not immediately reappear.
15. Elapsed time from host start/finish timestamps. Historical estimates use
    host-computed medians of at least five valid successful runs of the same
    routine in the bot's retained last 20 runs. Otherwise show No estimate.

## Ownership, compatibility and platform scope

The host owns queue execution/cancellation, Git subprocesses and timing
statistics. The desktop sends intents and renders bounded snapshots. New
`islandWorkspace`, `queuedMessages` and `cancelQueuedMessage` methods require
Local or authenticated Control access, like history/library/retry methods:
authorized remote computers can be selected in the island too. Screen-only
and pairing callers are rejected. No protocol fields are renamed or removed;
older hosts show unsupported-resource errors while supported reads survive.
Compatibility minimums are unchanged.

Queue snapshots include waiting user messages only, in their actual relative
order; they do not claim to display all background queue items. Cancellation
is serialized in the actor, persisted before cached text is removed, and
returns false if the work already started. Transcript deletion is not used.

Artifact/file navigation stays pending until the selected thread mounts.
The island stays visible on resource errors; it opens the app and closes only
after the requested published destination is validated and queued. Older
registered files outside loaded history ask the person to open the conversation
first. No arbitrary path read API was added.

Inspected corresponding iOS composer/store/artifact/retry surfaces, Android
artifact/store surfaces and the terminal library. These integrations apply to
the desktop's movable OS overlay on macOS, Windows and X11 Linux. The other
clients retain their existing conversation, library, drafts, routine and usage
surfaces; they have no equivalent floating desktop window, global Electron
shortcut owner, monitor docks or island-only font settings. iPhone system Live
Activities remain governed by their system container. Wayland remains
unsupported. The shared host capabilities are additive and reusable.

Version 2.44.2 is inherited and already ahead of latest fetched tag v2.44.1;
there is no second bump. Reconcile once with the eventual integration release.
XcodeGen regeneration from the preceding usage-fix handoff remains pending on
macOS. No phone source change was made in this batch.

## Review and validation

The user explicitly excludes builds, packaging, app launches, Computer Use,
visual tests and native compilation. Node code tests and `tsc --noEmit` are
allowed. Native Rust regression tests are authored but not executed because
their runner compiles code. Native compilation and UI behavior remain
unverified, not passed. No task app/host process was started and no Computer
Use settings were changed.

Three read-only reviews covered correctness, completeness and cleanup. A
fourth pass verified the findings against the final code:

- Fixed stale async owner/disposal races and errors leaking to another bot.
- Fixed a global mutation guard dropping navigation/draft sends during slow
  cancellations; regression coverage exercises simultaneous actions.
- Added timing reads when Activity's selected bot changes.
- Added successful resource-navigation acknowledgements so older-file failures
  stay visible rather than opening the wrong conversation silently.
- Preserved unsaved shortcut form edits when unrelated settings change.
- Fixed old shortcut callbacks remaining live after re-registering the same key.
- Refuted loopback-only access: Control access is intentional for remote bot
  selection and is covered by permission assertions.
- No cleanup findings justified expanding the change.

Final code checks:

- Desktop `npm test`: 630 total, 606 passed, 24 skipped, zero failures.
- New island regression files: 34 tests passed in a separate focused run.
- Desktop `npm run typecheck` (`tsc --noEmit`): passed; no binaries emitted.
- Rust formatting on all changed Rust files and `git diff --check`: passed.
- Seven new Rust tests cover queue selection/order and Git/timing parsing;
  permission assertions also cover the new APIs. Not executed, per the native
  compilation restriction.

One intermediate full Node run failed in the unchanged VNC bridge test file;
its detailed failure was not retained in that run's tail output. An isolated
rerun passed both VNC tests, and the final full suite above passed. No cause
was established, so this is recorded as an intermittent validation result,
not classified as a proven preexisting defect.

## Fifteen further integrations

These are proposals, not implemented in this batch:

1. Linked GitHub pull-request state and review requests.
2. Linked issue status and assignment changes.
3. CI/check results for the selected project's pull request.
4. Existing terminal session exit status and open-session action.
5. Remote computer connection recovery and reconnect action.
6. Upcoming routine occurrences from the existing schedule preview.
7. Memory review requests linked to the existing memory surface.
8. Connector sign-in/expiry states linked to existing authentication flows.
9. Published artifact revision notices opening the changed revision.
10. Delegated bot requests waiting for a response.
11. Pending-approval age using existing host timestamps.
12. File upload progress from the existing sending store.
13. Voice transcription/dictation state with microphone ownership respected.
14. Quick capture into the existing bot scratchpad.
15. Saved searches and bookmarks linked to their existing destinations.
