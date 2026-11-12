# Island usage corrections and integration handoff

Branch: `fix/island-usage-dots`. Base: `406db4b0f576c6d5470fc07aa8bfac5211d46ce9`.

The user requested code corrections and code tests only. Do not treat this
handoff as visual approval: no application build, installation, screenshot test
or app E2E was run. The user will confirm the appearance before another agent
integrates the branch and implements further improvements. Nothing is merged
or pushed by this task. `origin` has no `dev` branch; the task branches from the
current checkout's commit instead of rebasing the shared checkout.

## Changes

- Load the island Usage stylesheet from its window entry. Its omission caused
  the header meter and pager to stack vertically and left provider content
  without its layout rules.
- Let provider cards occupy the remaining height below the pager. Put labels
  and percentages above each dotted meter, with resets below. Long text wraps;
  additional windows scroll inside a keyboard-focusable region. Unavailable
  providers show the existing account status explanation.
- Keep the island's usage tint visible when the main app uses the light theme.
- Apply dotted meters to desktop Usage (including sidebar, settings, tray and
  island), iPhone Usage and widgets, Android Usage and widgets, and terminal
  Usage including its header. Keep each client's existing warning colors.
- Render Android widget bitmaps at their content dimensions to avoid stretching
  round dots. Keep zero usage empty and clamp invalid percentages in the new
  meter geometry.
- Replace the iPhone Lock Screen circular Usage gauge with a dotted ring inside
  the native widget container. Android has no corresponding circular accessory
  family; its shared linear Usage surfaces receive the dotted change.
- Prepare version `2.44.2` and bilingual phone release notes. This is ahead of
  the latest fetched tag, `v2.44.1`; reconcile with the target integration branch
  if another release has already advanced the version.

## Validation and remaining integration work

- Desktop `npm test`: 540 passed, 24 skipped, zero failures (564 total).
- Targeted island, usage and style dependency tests: 16 passed.
- Desktop `npm run typecheck` (`tsc --noEmit`): passed; no build artifacts emitted.
- Rust formatting and `git diff --check`: passed.
- After-change review confirmed and fixed the missed iPhone circular accessory
  gauge. Correctness and cleanup reviews found no other actionable defects.
  The static design scan reported five warnings on unchanged spring/shape
  animations in `island.css`; the existing island motion is deliberately kept.
- New Swift, Kotlin and Rust regression tests are included but were not executed:
  their runners compile native code, which the user explicitly excluded.
- XcodeGen is unavailable on this Windows machine. On macOS run
  `xcodegen generate --spec apps/project.yml` before integration; do not edit
  `project.pbxproj` manually.
- After the user confirms the design, build and test the affected desktop,
  iOS, Android and terminal clients, including installed widgets and long,
  localized labels. Native compilation and visual behavior remain unverified.
- No task-owned application or host was started. The Computer Use channel was
  restored to its original configuration; there are no Computer Use changes in
  these commits.

## Fifteen proposed island improvements

These proposals are implemented by the follow-up branch `feat/island-control-center`;
see [its integration handoff](island-control-center-handoff.md). They remain listed
here to preserve the original scope of the usage correction.
Client views should render existing host state and send existing intents;
missing shared logic belongs in the host.

1. **Edge placement:** choose top, bottom, left or right, adapting the shape,
   layout, open direction and pointer hit area to the selected edge.
2. **Drag and snap:** move the island between edges, with a lock to prevent
   accidental moves and a saved position.
3. **Safe placement:** respect the Windows taskbar, macOS menu/notch and display
   work area so island controls remain reachable.
4. **Monitor selection:** extend the current automatic, primary and pointer-follow
   choices with an explicit display selection and saved edge per display.
5. **Compact density:** offer compact and expanded layouts that preserve the same
   information and keep long provider names readable.
6. **Pinned providers:** choose which existing usage providers and windows appear
   first, while keeping a visible route to every other provider.
7. **Usage freshness:** expose the existing refresh action, last update time and
   stale/offline state directly in Usage.
8. **Quota notices:** surface existing usage limits and reset times as quiet,
   optional notices before a provider reaches its limit.
9. **Approval queue:** show the pending count and navigate existing permission
   cards without replacing the current approval choices.
10. **Unread conversations:** open chats with unread messages directly from an
    island list, using the same read state as the main app.
11. **Routine progress:** display the current step and result of existing routines
    and flows, with a shortcut to their detailed execution view.
12. **Channel activity:** surface relevant channel mentions and link to their
    existing conversation or triggered flow.
13. **Voice call controls:** show active call state and the existing mute/end
    controls, alongside the island's current message dictation.
14. **Connection and health:** expand the existing health summary with host/relay
    connectivity and a shortcut to the current diagnostics screen.
15. **Focused bot actions:** select a working bot and expose its existing
    stop/open-conversation actions without leaving the island.
