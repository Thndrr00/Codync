# Upstream PR integration batch

Integration branch: `integrate/upstream-agent-desktop-screen`.
Fork base: `e60772d2` on `Thndrr00/Codync:main`. Source feature version: `2.20.0`.
This branch is for review and acceptance; it does not publish a release or merge
the batch into `main`. The original checkout's uncommitted documentation is separate.

Feature patches were applied against their upstream merge bases and adapted to
the fork. Upstream branch ancestry was not merged wholesale. In particular,
PR 105's unrelated merge history and credential-vault replacement were excluded.
Upstream version bumps and PR 132's signing/packaging changes were excluded.

## Imported changes and provenance

- [113: searchable model picker](https://github.com/leepokai/Codync/pull/113),
  `18653091bc7d5a5e579f68f24437d7e87f55b89e`, Serhii Londar.
  Keeps the search field above the scrolling list on desktop and iOS. Android
  gets an equivalent searchable sheet. Default and saved model choices survive
  filtering. Manual IDs, model options and Fast mode remain available. The TUI
  already supports entering a model ID directly and cycling advertised models.
- [133: Windows SSH](https://github.com/leepokai/Codync/pull/133),
  `5da2d1433b0f7321723d6c93e3ce4d320fe84d2f`, Sean Ferguson.
  Uses native OpenSSH, Windows path handling, hidden console processes and
  listener ownership checks. Retains the fork's isolated sandbox known-hosts
  paths and cleanup boundaries. Windows requires OpenSSH Client to be installed.
  Phones and TUI do not manage the desktop application's SSH profiles.
- [157: native Hermes](https://github.com/leepokai/Codync/pull/157),
  `7ac960e814c3fc7a2494b89e4f40571b5032b050`, Ryan.
  Starts Hermes through its native ACP command and exposes setup and executable
  discovery through the host's shared backend catalog. All clients use that catalog.
  No provider is installed or account authenticated by applying this branch.
- [156: routine unread counts](https://github.com/leepokai/Codync/pull/156),
  `622ae2078d0cca1c6398645d4ede8bc083131375`, Kai Wu.
  Tags final routine results with the routine run and counts them once in host
  unread queries, including threaded results. All clients consume the corrected
  host count. Preserves existing thread and routine lifecycle behavior.
- [160: non-interactive send CLI](https://github.com/leepokai/Codync/pull/160),
  `f9a4a49660cba12cd9a5e3e7e61021855f595218`, Ryan.
  Adds `codync-host send --bot <id-or-name> --wait --json "message"`, file/stdin
  input, thread selection and explicit exit codes. Uses the local authenticated
  host API. Waiting excludes approval time; a timeout does not stop the bot.
  No extra graphical button is necessary: this is a local automation entry point.
- [105: isolated Antigravity accounts](https://github.com/leepokai/Codync/pull/105),
  `2eb7d112cc25666e94c0f739059d178db819e267`, Rafa-Smart.
  Adds an optional host-local account ID to a bot. The host creates isolated
  Antigravity profiles and injects `GEMINI_HOME` into agent sessions and model
  discovery. Profiles explicitly select personal OAuth and force file storage
  (`AGY_ACP_FORCE_FILE_STORAGE=1`) so macOS does not reuse a shared keychain entry.
  Dedicated processes remove inherited or saved Google API/ADC overrides.
  Account changes restart sessions with fresh context. Desktop, iOS,
  Android and TUI expose account selection and sign-in. The TUI uses its account
  field: arrows select, `i` signs in, `r` reloads. Empty selection uses the default
  profile. IDs and profile metadata are validated; path traversal, symlinks and
  Windows reparse points are rejected. Profiles in use cannot be deleted.
  Credentials stay on the host. They are not part of exported bot templates.
  The fork's existing vault remains unchanged. Browser sign-in still requires
  the user's interaction on the host computer. Live multi-account authentication
  must be accepted against an installed compatible Antigravity adapter.
  Configuration follows the [Antigravity ACP integration reference](https://github.com/openclaw/acpx/blob/main/agents/Antigravity.md).
  Sign-in has a single 600-second host deadline; clients allow 660 seconds.
- [135: compatible speech endpoints](https://github.com/leepokai/Codync/pull/135),
  `329d508edf2d2211f03b2f46ffdf63daf636e0b2`, Cratgog.
  Supports `CODYNC_OPENAI_BASE_URL` for host-mediated speech transcription and
  synthesis. Accepts HTTPS and loopback HTTP, normalizes `/v1`, rejects embedded
  credentials/query/fragment and disables redirects. Validates a supplied key
  against that endpoint. The custom endpoint does not expose Realtime models;
  Realtime requests fail explicitly instead of using credentials on another host.
  The shared host implementation serves all clients using these speech calls.
- [90: process and session ownership](https://github.com/leepokai/Codync/pull/90),
  `d3f5a50f27a0f2d6e339a4cc64434036fb47f6f1`, Sean Ferguson.
  Owns descendants with Unix process groups / Windows Job Objects, starts Windows
  children suspended until owned, and cleans up failed or abandoned allocations.
  Persists new session identity before model setup, preserves initial profile and
  thread instructions until the first successful prompt, and keeps resumable and
  nonresumable session policy. Adaptation retains the fork's memory snapshots,
  model options, Fast mode and routine metadata.
- [40: trickle ICE](https://github.com/leepokai/Codync/pull/40),
  `ad54454350fe3d833c770ae98457d427b2726e49`, Sean Ferguson.
  Adds bounded, session-scoped candidate subscriptions and serialized candidate
  exchange in the host and macOS/Linux helpers. iOS and Android negotiate the
  capability and retain complete-SDP fallback for older peers. Candidates wait
  until the answer is applied; completion follows candidates. Signaling loss
  during negotiation fails the attempt; established media can detach signaling.
  Device permissions and authorization still control access. Cloud/relay already
  transport the encrypted calls/subscriptions and need no separate deployment.
- [132: SSH screen viewer](https://github.com/leepokai/Codync/pull/132),
  `c672361115b44b156872c7b9c183b3518c32475a`, Richard.
  Adds a desktop noVNC window from Computers and bot Details. The SSH connection
  forwards remote loopback port 5900. The bridge uses the maintained `ws` parser,
  bounded frames and stream backpressure instead of the prototype frame parser.
  Main-process authorization, private capability URLs and attachment ownership
  restrict access. This remains a prototype with an external plain RFB server
  requirement; it is separate from the mobile WebRTC viewer. It does not add a
  Windows capture helper. TUI has no video surface.

## Acceptance and release

Automated results and remaining environment limitations are recorded below after
the integration checks. A successful build alone does not establish live speech,
OAuth, SSH, remote capture, TURN or physical-device acceptance.

Before release, regenerate the Xcode project on macOS with
`xcodegen generate --spec apps/project.yml`; never hand-edit `project.pbxproj`.
Run Swift package tests, macOS helper tests and the Linux helper tests with their
native system libraries. Install fresh client/host/helper builds and exercise:

1. Model filtering, selected/default choices, options, refresh and empty/error states.
2. Two Antigravity accounts and two bots; switch account, discover models, restart
   and confirm each profile's identity, then reject deletion of a used profile.
3. Windows SSH connect, host-key trust, selected identity, retry, occupied port,
   disconnect and application quit. Existing trusted production profiles stay separate.
4. SSH/VNC first frame and input, reconnect, bridge rejection and shutdown.
   Use a supported remote graphical environment and loopback-only RFB service.
5. Mobile remote screen on local and separate networks, late ICE candidates,
   ordering, cancellation, permission revocation and old/new peer fallback.
6. Routine results counted once; CLI success, approvals, failure and timeout;
   session/model setup failure and process-descendant cleanup.
7. Compatible speech transcription/synthesis with its intended endpoint; invalid
   endpoints, redirects and unsupported Realtime fail without sending credentials elsewhere.

The required native desktop/phone end-to-end checks must pass before treating this
branch as release-ready. Roll back by excluding/reverting this integration branch;
do not reset another contributor's worktree or erase saved account profiles.
Before downgrading an installed host, stop bots using dedicated accounts: older
hosts do not implement the account selector and use their default agent profile.

## Review corrections

The integration's read-only review found two account portability issues: desktop
and Android sign-in deadlines differed from the host, and their templates retained
host-local account IDs. Both were corrected, with template regression coverage.
The imported session regressions also exposed a missing explicit reset of an
uncertain nonresumable allocation; resetting now releases that connection before
starting a replacement. Startup test failures kill and reap their own child host.

Cleanup moved the new iOS searchable picker, voice endpoint policy/tests and
desktop template encoder into separate modules. It removed unused SSH wrappers
and kept account-aware model/Fast-mode keys consistent in the TUI. Broader runtime
and store rewrites were skipped to preserve the fork's existing memory, session
and transport behavior during this integration.

## Recorded validation: Windows, 2026-10-20

- Rust formatting and `cargo clippy --all-targets -- -D warnings` passed.
  The full suite passed with 420 successful tests and one ignored test, using
  `cargo test --manifest-path host/Cargo.toml -j 2 --no-fail-fast -- --test-threads=1`.
  This includes 329 host unit tests, the Windows launcher test and 90 integration
  tests covering allocation, session setup, Fast mode, pairing, encrypted chat,
  files, memory, routines, CLI, collaboration and turns. Parallel host startup
  initially exceeded this machine's deadline; the complete serial run passed.
- Desktop TypeScript checking and the production renderer/main/preload build
  passed. The complete Node suite reported 140 passing tests, 19 platform skips
  and 11 failures in the existing update fixtures. Those same 11 failures were
  reproduced against the fork base in an isolated source copy; the integration
  does not change the updater or host-controller implementation. The new VNC,
  Windows SSH, account model and portable-template regressions passed.
- Android final core suite: 133 tests, zero failures/errors, one skipped test.
  App suite: 31 tests, zero failures/errors. `assembleDebug` and `lintDebug`
  passed; lint reported zero errors and 77 warnings. The final debug APK is
  `apps/android/app/build/outputs/apk/debug/app-debug.apk`.
  Thirteen new tests cover candidate validation and signaling order, bounds,
  cancellation, detachment and native rejection.
- Android's unchanged slow-consumer catch-up test intermittently exceeded its
  eight-second deadline. The exact `e60772d` baseline reproduced that timeout
  using the same JDK and one Gradle worker. The final integrated isolated case
  and complete core suite passed without changing its timeout or coverage.
- The built host reports `2.20.0`; `codync-host send --help` exposes the new CLI.
  Staged whitespace checks passed. Cleanup and a separate read-only refactor
  review completed; both account portability findings were verified as resolved.

Native acceptance remains open:

- Windows desktop UI automation could not connect to the Computer Use native
  pipe. A passing Electron build is not a visual end-to-end check.
- `adb` had no connected device; the Android APK was built but not installed
  and exercised on a physical device or emulator.
- Swift/Xcode/XcodeGen and usable macOS/Linux test environments were unavailable.
  iOS tests, native helper builds/tests and Xcode project regeneration remain
  required before release.
- No live Antigravity OAuth, Hermes installation, compatible speech service,
  trusted SSH/VNC computer or cross-network TURN session was exercised. These
  require the services/devices in the acceptance checklist above.

This is a reviewed integration branch with recorded environment limits, not a
production acceptance claim. The original checkout's documentation and `main`
were not modified by this integration.
