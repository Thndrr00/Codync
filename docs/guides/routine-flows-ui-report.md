# Routine flows, screens: report

Branch `feat/routine-flows-ui` (from `feat/routine-flows-host`). Feature: [routine flows](../features/routine-flows.md).
Host report: [routine-flows-host-report.md](routine-flows-host-report.md).

**Nothing in this branch has been compiled, type-checked or run.** At the user's request the code was written first and the
gates are left for one pass at the end together with other additions. Every statement below about behavior is therefore a
design statement, not a test result.

## Scope

| Client | State |
|---|---|
| Desktop (Electron/React) | Written, not built |
| Android (Compose) | Written, not built |
| Terminal | Done in the host branch; vocabulary checked against these screens (same statuses and words) |
| iPhone / Mac native | **Not done, by instruction** (no iOS work). CLAUDE.md asks for every client in the same change: this is a deliberate gap, and the iPhone has no Flow screen yet |

## Decisions

- **Desktop canvas: hand-written, no library.** Openstark's board is hand-written too (no graph library in its dependencies). Needs are small
  (dozens of nodes, bezier edges, pan/zoom/drag), and a library such as React Flow would add a dependency and its own
  stylesheet and inline-style model, against `desktop-styles.md`. The only inline styles are computed transforms.
- **Model apart from the views**: `lib/routine-flow.ts` (desktop) and `core/RoutineFlow.kt` (Android) hold types, automatic
  columns, edges, the list rows, the connect pre-check and the view math, with tests, so the views stay without logic. Both mirror the host's rules.
- **Events**: the host's `routineFlow` event carries no data; each client keeps the newest rev per bot (`flowRev` / `flowRevs`) and a canvas on
  screen loads again when it moves. Desktop's parser treats a malformed one as undecodable (the usual rewind); Android ignores it.
- **Connecting**: desktop by dragging from a bot's dot or from the inspector's menu; Android from the bot's detail sheet (explicit action).
  When several routines of the canvas allow the same link, a menu asks which routine.
- **Accessibility**: nodes are focusable buttons (arrow keys nudge a selected node on desktop); a list view with the same selection and
  inspector is the keyboard / TalkBack equivalent; Android defaults to it with TalkBack on or under 600 dp.
- **Link handles on Android**: edges are hard to hit with a finger, so each link has a small handle at its middle.

## Differences from Openstark

No agent chat panel, no "new agent" card on the canvas, no webhook configuration on the canvas (routines keep their own editor),
no auto-layout beyond columns by depth. Terms are Codync's (bot). The node card follows Openstark's content (avatar, name, model,
status, "Not connected" / "No runs yet") in this app's tokens. No Openstark file was copied.

## Gates

| Gate | Result |
|---|---|
| `npm run typecheck`, `npm test` (desktop) | **not run** |
| `./gradlew assembleDebug testDebugUnitTest :core:test lintDebug` | **not run** |
| E2E on desktop (`npm run dev`) and Android (signed APK, adb) | **not run**; screenshots: none |
| Host | unchanged by this branch |

## Known gaps to check on the first build

- Translations: new phrases are English until added to the shared language tables (under `apps/ios/Kit/.../Languages`, not touched).
- Desktop `package.json` test glob already covers `src/renderer/lib/*.test.ts`, where the model test lives.
- Android: `FlowCanvasView` mixes dp layout with a pixel-translated layer scaled by the view; drag deltas are divided by density only. Verify the feel on a device.
- The `BotAvatar`, `Palette.added`, `Type.system` and symbol names (`clock.arrow.circlepath`, `circle.dashed`, ...) were taken from nearby code; any missing symbol falls back as other icons do.
- Android `action("flow:<bot>")` runs one flow change at a time per bot: a move made while a load is in flight is dropped.
- The two host failures listed in the host report are still open.
