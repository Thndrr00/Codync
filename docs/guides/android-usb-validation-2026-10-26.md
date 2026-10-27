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

## Features added after the first validation round

- Remote mini-apps: the host method `appRequest` (`host/src/api/apps_bridge.rs`) runs one
  `/apps/{bot}/{slug}/{token}/...` request on the computer and returns status, content type and a
  base64 body over the E2E channel, with the same link-token check as a browser request. The phone
  shows the app under the virtual origin `https://codync-app.invalid`: its WebView answers page and
  file requests through `shouldInterceptRequest`, and a bridge script relays `fetch` calls (data,
  tells, errors) and replaces the `_live` socket with a poll of the app's collections
  (`MiniAppWebClient.kt`, `MiniAppBridgeScript.kt`). Loopback URLs are no longer a dead end.
- Proactive check-ins run in the host (`host/src/proactive.rs`): every minute it wakes each bot with
  them on, once its interval has passed, outside its quiet hours and within its daily limit. A bot
  just switched on waits one interval. Android and desktop edit the settings.
- Pending decisions read the entries the host really writes (pending permissions with their allow and
  deny options, `takeoverRequest` notices, error notices no later message followed) and Android and
  desktop answer permissions from the card. The old `passkey` / `error` entry kinds never existed.

## Features still requiring implementation

- Action-class guardrails (`actionRules`): the host stores them but nothing enforces them yet, so
  neither client shows controls for them.
- `appsInBackground` of the proactive settings is stored but not yet consulted when a check-in runs.
- iOS and the terminal UI have no mini-app, proactive or pending-decision views.

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
