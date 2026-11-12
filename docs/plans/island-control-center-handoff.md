# Island control center

Branch: `feat/island-control-center`, based on `3406f1dc` from
`fix/island-usage-dots`. Integrating the feature branch includes the previous
usage corrections. The shared checkout was left unchanged; `origin` has no
`dev` branch. Do not merge or push this work until the maintainer confirms the
appearance and the integrating agent completes native validation.

## The fifteen requested improvements

1. Edge placement: top, bottom, left and right, with edge-specific geometry,
   shape anchoring, wake strips and content layout.
2. Drag and snap: unlock the handle, drag, then release near an edge. Pointer
   cancellation and a main-process timeout release capture safely.
3. Safe placement: respect each display's work area. A centered top notch is
   the explicit Mac exception; X11 input shapes are bounded to the panel.
4. Display selection: automatic, primary, pointer-follow or a specific display;
   save edge/offset per display, and fall back if a saved display disappears.
5. Density: compact or expanded, with narrow/two-row headers and one usage
   provider per page when space is limited.
6. Pins: provider and limit pins reorder presentation without hiding other limits
   or modifying host reports.
7. Usage freshness: refresh, busy/error/offline state and last provider update;
   readings older than ten minutes are labelled saved.
8. Usage notices: optional threshold warnings and upcoming resets (15 minutes).
   These stay inside the island and do not send system notifications.
9. Approval queue: count, previous/next controls, unchanged allow/deny options
   and keyboard actions; answering a card does not close a multi-card queue.
10. Unread conversations: use existing unread counts, including group chats,
    and open the corresponding conversation.
11. Routine progress: recent host runs, status, detail, and steps associated with
    their actual run ID; open the existing flow canvas.
12. Channel mentions: bounded metadata-only host receipts for accepted Slack,
    Discord and Telegram messages, with real mention classification and a link
    to the assigned bot conversation. Historical messages are not backfilled.
13. Calls: publish the existing device session's state and dispatch mute/end to
    its callbacks. Commands bind to the exact call ID to reject stale controls.
14. Connection and health: existing host connection/route/resource data and a
    diagnostics shortcut; no separate client health logic.
15. Focused bot actions: select/cycle bots, open a conversation and use the host's
    existing stop intent. n8n's status/editor is also available under health.

## Ownership and compatibility

Electron main owns monitor geometry and persistent docking settings. Renderer
preferences own island-only pins and notices. The host owns accepted channel
receipts, queries and all existing execution decisions. No client duplicates
scheduling, agent routing, approval validation or microphone ownership.

The new `channelActivity` method is loopback-only. It accepts `limit` (1–50,
default 30) and `mentionsOnly` (default true). Retention is 200 receipts; queries
filter deleted/hidden bots and mentions before limiting. Receipts contain source,
bot/thread/channel identifiers, mention status and time, never message text,
sender IDs or credentials. Retries retain their original accepted timestamp.
Telegram checks exact mention entities using UTF-16 offsets, including emoji.

The island receives only n8n's state/error/MCP presentation fields, never its
session cookie, editor URL or filesystem path. Older hosts show unsupported
activity errors while existing Usage and bot actions remain available. These
are additive changes and do not raise compatibility minimums.

This change is desktop-specific on macOS, Windows and X11 Linux. iPhone,
Android and the TUI have no movable OS panel or monitor work-area geometry;
their existing Usage, approvals, unread, routines, channels, health and call
surfaces retain their behavior. The new host receipt API is internal to the
local desktop. No duplicate phone/TUI island is introduced. Wayland remains
unsupported, as before.

Version `2.44.2` is already ahead of the latest fetched tag (`v2.44.1`) in the
inherited usage-fix commits, so no second bump was made. Reconcile the final
release version with the integration branch. XcodeGen regeneration from the
previous handoff remains pending on macOS.

## Validation boundary

The user explicitly requested code and code tests only: no builds, app launches,
visual tests, simulator/emulator runs, packaging or deployment. Desktop tests
and `tsc --noEmit` are run without generating app binaries. New Rust receipt and
Telegram mention regression tests are included but not executed, because their
runner compiles native code. Native compilation and all visual behavior remain
unverified and must be checked by the integrating agent after user confirmation.

### Code verification results

- Desktop `npm test`: 596 total, 572 passed, 24 skipped, zero failures.
- Desktop `npm run typecheck` (`tsc --noEmit`): passed, including final docking
  and action-error changes; no application build artifacts were emitted.
- Rust formatting checks on all changed Rust files and `git diff --check`: passed.
- Static design scan: five warnings on the existing island spring and
  width/height animations. Those unchanged animations are retained deliberately.
- New Rust tests cover receipt retention, redelivery, visibility, filtering and
  Telegram UTF-16 mention ranges. They were not run because compilation was
  explicitly excluded.

Initial delegated review confirmed two host defects: Telegram username-prefix
false positives and replayed receipts appearing new after retention. Both were
fixed and received regression tests. Code tests also reproduced and fixed stale
connection errors, cached runs crossing computer ownership, and stale call
commands controlling a replacement session. The larger toolbar sits below a
camera notch, and the first native window bounds use the saved safe dock before
the window is shown.

The final delegated review attempts hit the service usage limit. The coordinator
completed correctness, completeness and cleanup review directly; this does not
substitute for the deliberately deferred native and visual validation. No task
application or host process was launched, and no Computer Use configuration was
changed. The branch is committed locally for integration; it is not merged or
pushed.

## Fifteen further ideas

These are new proposals, not part of the implemented change.

1. Search bots and island actions from a compact command field.
2. Assign configurable keyboard shortcuts to the island's existing actions.
3. Filter activity by the selected bot or project.
4. Retry a failed routine from its activity row using the host's retry action.
5. Show queued work and its order, with existing host cancellation actions.
6. Preview the selected bot's most recently modified files.
7. Open recent artifacts produced by the selected bot directly from the island.
8. Show the project's current Git branch and pending changes from host data.
9. Resume a saved message draft without switching to the main app.
10. Offer an island presentation profile for focused/full-screen work.
11. Add independent text-size and high-contrast options for the island.
12. Show a compact daily work summary from existing host counters.
13. Keep a short, dismissible history of island errors and notices.
14. Group repeated notices into one row with a count.
15. Show elapsed time for active runs, with estimates only when host data
    supports them.
