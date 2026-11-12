# Notch island (desktop)

The desktop app shows working bots in a black shape at the top center of the screen. On a
MacBook it grows out of the notch; on a Mac without a notch, and on Windows and Linux, it hangs
from the top edge. It draws Codync's own bot characters. Design inspired by
[@novra.design](https://www.instagram.com/novra.design/).

Turn it on or off in **Settings → General → System** (*Show in Notch* on macOS, *Show at top
of screen* elsewhere) or the menu bar / tray menu's **Settings**. It is off by default while it
is experimental. The setting belongs to the desktop installation (`showIsland` in `host-prefs.json`).

**Where it sits** (tray menu → Settings → Island position): the display with the notch (else the
main one), the display under the pointer (it moves when the pointer crosses to another screen),
or this computer's main display. The choice is saved as `islandPlacement` beside the switch, so a
laptop with an external monitor can keep it on the screen being used.

## What it shows

Open, the island fits its display work area, with a toolbar below the camera notch.
Workspace, Bots, Message, Usage, Activity, Status and Options share the same scrollable
content area. Compact or expanded density changes the available content height. The card holds the bot in focus as a big character with a glow and a state
badge; alerts tint the card from the bottom (amber, green, red). The other bots sit in a column
on the right.

The working badge is Codync's thinking orb. The usage meter and the Usage tab show the tightest
local plan limit and every limit with its reset (amber from 70 %, red from 90 %, like the menu
bar). A bot can be messaged from the island, and ⌥Y / ⌥N answer an approval from any app.

A working character follows the pointer with its eyes: the openings move smoothly (`tanh` of the
distance, so they ease toward the edge of the face instead of snapping) and the whole character
breathes by about 1 %. An idle character is drawn once and never tracks, so a roster full of them
costs no frames; with Reduce Motion on, nothing moves at all.

| State | When | What it shows |
|---|---|---|
| Hidden | No bot works or waits | Invisible over the notch (a thin strip at a plain top edge still wakes it) |
| Peek | The pointer reaches the bare notch | The notch a little wider, the bot in focus peeking out; opens after 650 ms |
| Compact | A bot works or needs you | The notch widened: the bot in focus with its badge on the left, the others (or an orb) on the right |
| Overview | Pointer rest or a click, bots working | The bot in focus with its last three tool calls (newest brightest) and the other bots as pills; each opens its conversation |
| Nothing running | Opened with no bot working | "Nothing running right now." and Message a bot |
| Approval | A permission card is pending | The command, what an edit changes (`+N −M` per file, the changed lines on click) and the card's options, deny left and Allow once right (⌥N / ⌥Y from any app while it shows) |
| Dropping files | A file drag reaches the shape | A dashed outline and "Drop to send to *bot*"; the files are read here and sent to the bot in the big slot (or the one that needs you) |
| Finished | A bot stops working | "*Bot* is done" with its last message, Open conversation / OK, for 5.2 s |
| Failed | A bot stops on an error | "*Bot* stopped." with the error in red, until OK |
| Message a bot | The header's bubble, or the [voice input shortcut](voice-call.md#voice-input-shortcut) | The bot chip (click for the next bot) and a field with the mic (macOS); Enter sends, Esc closes. Until the shortcut is first used, a tip under the field says how to talk from any app |
| Usage | The header's gauge or the usage meter | The providers closest to a limit first, two to a page side by side with arrows (and "1 / 3") between pages: each window's bar, percent and reset, or its note when it has no percent |

Opened by hover, the island folds 0.8 s after the pointer leaves; opened by a click or a tab,
10 s after (a 2 pt line counts down). The message field stays until it sends, Esc or a click in
another app; switching to another tab while it dictates stops dictation and keeps the words. An approval opens by itself and stays until answered; the Bots tab puts it off
until another arrives. Answers and messages go through the chat window (`respondPermission`,
`send`), like the chat's own cards and composer. The states, sizes and motion are drawn in the
design canvas the island was built from.

Every open view is the same size, so switching tabs or an alert taking over never moves the
black shape: the next view cross-fades over the last. When another bot takes the focus it
springs in out of a blur, a bot joining the pills or the column does the same, and the others
slide to their new places.

## Workspace and focused controls

Workspace searches bots by name, backend, status or project path and searches existing island
actions. Selecting a bot reads its recent library, completed edit patches, actual queued-message
order, Git context and the computer's trailing 24-hour counters. Read-only previews contain at most
five completed tool edits, three files per edit and 4,000 patch characters per file. Proposed
permission edits are excluded. Registered files and artifacts open the existing conversation
viewers; arbitrary filesystem paths are never read by the island. Older registered files whose
entry is outside loaded history ask the person to open the conversation first.

Cancel removes one waiting user message through the bot actor; it never stops the current turn
or unrelated queued work. If the message started meanwhile, the host reports that it was not
cancelled. This list shows user messages, not a claim that background routines occupy no queue
positions. Retry uses `retryRoutineRun` and its original event, only when the host reports that
run as retryable. Activity filters by bot or exact workspace path on the same computer.

Resume draft explicitly transfers that saved draft to the island message field without opening
the main window. Closing without sending keeps the original saved draft. Sending uses the normal
recoverable send flow and clears the saved original only if it is still unchanged; concurrent
main-window edits survive. The island never includes draft bodies in its routine summaries.

Elapsed times use host start/finish timestamps. Historical routine estimates are the median of
at least five successful, timed executions of that same routine in the host's retained last 20
runs for the bot. Queued time, failures and missing timestamps are excluded. An estimate is a
historical duration, not a promise of completion time; otherwise the UI says No estimate.
The daily summary is explicitly Last 24 hours, not a calendar-day count.

Options has independent text size (11–18), high contrast, and an optional Quiet profile. Quiet
suppresses automatic hover opening and completion/error popups, preserving approvals and manual
controls. Fullscreen quiet detects other Codync windows only; use manual Quiet for other apps.
Global shortcuts are opt-in for show/hide, search, activity, message, usage and Quiet. Approval,
editing and existing application shortcuts are reserved. Unavailable bindings show errors;
disabling the island unregisters only its own keys.

The notice history is local device UI state: at most 50 grouped items and 200 active keys.
Unchanged polling does not increment counts; a notice recurring after recovery does. Clear
history leaves active notices acknowledged so they do not immediately return. It stores short
error/notice text and bot references, not conversation bodies or credentials.

## How it works

- **Window** (`apps/desktop/src/main/island.ts`): a transparent, frameless panel bounded by
  the selected display work area: up to 720 × 380 horizontally or 420 × 560 vertically.
  Options selects an edge, a display and density; unlocked dragging snaps to the nearest edge.
  Positions are saved per display in `islandControls`. macOS:
  a non-activating panel at the main-menu level + 3, so it covers the menu bar, every Space and
  full-screen apps. Windows: a toolbar window (out of Alt+Tab). It never resizes; the shape
  animates inside it with CSS (spring open, plain close).
- **Click-through**: the window takes the mouse only over the shape (+8 pt); the renderer
  reports the shape's target rect on every change. macOS and Windows: the main process polls
  the pointer (60 Hz near the panel, ~7 Hz elsewhere), toggles `setIgnoreMouseEvents` and tells
  the renderer when the pointer enters or leaves. Linux (X11) only reports the pointer over the
  app's own windows, so the window is shaped to the island instead (`setShape`, kept at the old
  outline until a shrink's animation ends) and the renderer's own mouseenter/mouseleave is the
  hover.
- **Notch size**: Electron has no notch API, so `osascript -l JavaScript` reads each
  `NSScreen`'s `safeAreaInsets.top` and `auxiliaryTopLeftArea` / `auxiliaryTopRightArea`; the
  notch is the width between them (a 14" MacBook Pro: 185 × 32 pt). It is read again when
  displays change.
- **Data**: the chat window owns the stores. `useIslandSummary` (next to the tray's summary)
  sends bots and pending permission cards when they change; the island window renders them
  (`views/island/`, state in `IslandModel`, pure rules in `shared/island.ts`). Buttons travel back
  as `WindowCommand`s (`openBot`, `respondPermission`, `sendMessage`, `sendFiles`) to the chat window, which
  calls the host. The island has no host connection of its own. A drop sends the OS paths of the files
  (`webUtils.getPathForFile`); the chat window reads them (at most 20 files, 100 MB each, like the composer) and
  sends them with no message of their own. It takes the keyboard when
  the message field opens (`setFocusable`) and gives it back once the island has folded
  (giving it back takes the window off screen for a moment on macOS); ⌥Y / ⌥N are global
  shortcuts only while an approval shows.

## Platforms and other clients

- **Linux on Wayland**: not available (a window can't place itself at the top or stay above
  others), so the setting is hidden. On X11 the transparent corners need a compositing window
  manager (GNOME, KDE and most desktops have one); without one they show as a solid margin.
- **Android**: no desktop-style floating OS island. Existing conversation, library, drafts,
  routine, activity and usage screens retain their behavior.
- **iPhone**: Live Activities and the Dynamic Island already cover this
  ([notifications and Live Activities](../design/push-and-live-activity.md)); no change.
- **Terminal UI**: a terminal can't draw over other apps; its roster already shows working and
  waiting bots.
