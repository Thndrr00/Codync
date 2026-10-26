# Android USB integration validation — 2026-10-26

Worktree: `Codync-windows-validation`, branch `integrate/windows-all-additions`.
The comparison base remains `origin/main` at eded5f5f. Source worktrees and
uncommitted original branch work were preserved.

## What remained from the existing work

The old Android/Windows integration snapshot was compared with the current
integration by behavior, not merged wholesale. Its missing Android runtime
features were the durable analytics queue, consent-safe delivery cancellation
and onboarding abandonment tracking. These have been ported while preserving
the newer typed events, session metadata and existing onboarding completion.
Other inspected Windows differences were older views or compatibility modules
already replaced in the current code, not additional features to restore.

Additional cross-client defects found during acceptance preparation were fixed:

- Android and desktop share the freshly issued mini-app capability URL rather
  than the owner's URL or an origin-less fallback.
- Android published-app notices tolerate older hosts omitting `botId` by using
  the containing entry's bot identity.
- Android displays loading/API/share/mutation errors instead of empty success.
- Android's lock summary uses only counts and freshness, not ordinary bot names.
- Android supports editing bookmark notes through the existing host method.
- Explicit Android agent/model choices survive connection/hello refreshes.
- The real roster creation route opens the new bot/group forms. Physical testing
  found that these forms existed but the root still opened the old settings
  editor; the corrected routing is built, awaiting device installation.

## Features still requiring implementation

- Remote mini-app rendering: the current host supplies loopback browser URLs.
  Android now identifies these as computer-only instead of opening the phone's
  loopback address. USB reverse must not be used as evidence of remote support.
- Action-class guardrails and proactive check-ins: the host stores these fields
  but has no enforcement/scheduler consumers. The active controls were removed
  from Android and desktop, with an explicit unsupported message. Existing role
  templates still set their valid instructions and appearance.
- Pending-decisions inspection still needs actual host notice aggregation and
  corresponding Android actions; old synthetic `passkey`/`error` entry kinds
  are not evidence that this works with real host entries.

These items are not counted as working merely because their views compile.
iOS and terminal have no corresponding mini-app/Sentinel/action-rule renderer
affected by these specific fixes; Android lock widgets are platform-specific.

## Device and validation status

The connected Xiaomi M2010J19CG runs Android 12/API 31. Its existing Codync Dev
2.25.0 and the new APK use the same signing certificate. Upgrade uses
`adb install -r`, retaining accounts, identity and saved data. Ordinary reset
instrumentation is excluded on this personal phone; only the explicitly
read-only `LiveRosterTest` probes may run against its existing connection.

- Desktop TypeScript and 333 tests pass; 19 platform-specific tests skipped.
- The installed Windows share flow creates a fresh view token and copies its
  full URL through the native clipboard bridge. The copied URL reads with HTTP
  200 and rejects a collection write with HTTP 403. Clipboard contents match.
- Host runtime source is unchanged in this round; previous native/host results
  remain applicable.
- Android: 223 core tests (one skipped), 80 app tests, both debug APKs and lint
  pass. The final app/test/assemble/lint invocation completed in 8m45s.
- The initial combined build hit the unchanged slow-consumer channel deadline
  while packaging/compilation were active. The isolated full core rerun passed
  in 34s without changing that deadline.
- First upgrade to 2.26.0-dev succeeded, retaining the existing account and bot.
  The saved connection was offline; a matching Dev pairing was added using
  encrypted transport over `adb reverse` on port 19223. Other direct candidates
  and the previous computer were retained. The phone displayed Connected and
  the Windows host's roster.
- Physical UI passed: catalog browsing/details; creating Code reviewer from its
  profile; installing Software team with its coordinator, three collaborators
  and their four-member group. This does not prove real provider delegation.
- Physical UI failed before the routing fix: New bot opened the old settings
  form without the new Name/purpose/suggestions flow. The fix is compiled, but
  MIUI rejected its installation and the auxiliary test APK with
  `INSTALL_FAILED_USER_RESTRICTED: Install canceled by user`.
- The latest corrected APK is prepared in phone Downloads as
  `Codync-Dev-2.26.0-usb-20261026.apk`. Installation and the creation retest await
  the owner's device approval. No PIN/password or removal of data is needed.
- Remaining physical checks include new bot/group creation, explicit unavailable
  agent selection, actual provider chat, note editing, diagnostics, mini-app
  computer-only state/sharing, routines, widgets and offline recovery.
- No ordinary reset instrumentation was run. The optional read-only roster and
  creation probes remain unrun because MIUI rejected their auxiliary APK.

The original bot was preserved. Six task-created catalog/team bots are recorded
separately under `build/android-usb-e2e/task-bots.json`, along with a tiny test
mini-app and editable bookmark in the task bot's own workspace. They are retained
for the pending phone checks; cleanup must never delete baseline bots.

Task-local evidence is kept under `build/android-usb-e2e`. Pairing links,
identity secrets and authentication tokens must not be included in this report.
