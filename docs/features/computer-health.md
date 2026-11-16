# Computer health

The host reads the computer every 10 seconds (`sysinfo`): memory, CPU, the disk the host keeps its data on, and the memory
of every agent process Codync started (the host knows their pids). `hostHealth` returns the last reading; clients read it
again every 10 to 15 seconds.

## Policy

Memory in use against two levels the person can change (`setHealthThresholds`, default 70 % and 85 %):

| Level | What happens |
|---|---|
| below 70 % | normal |
| 70 % and up (warning) | each chat with a running agent gets one notice that memory is getting full |
| 85 % and up (limit) | no new turn starts: the message fails with "Not started: this computer's memory is N % full, above the 85 % limit…" (the turn is never launched, so a person can send it again once memory frees up); idle agents give their memory back at once (`Cmd::FreeMemory`, the idle release without waiting out its 30 minutes); busy bots are left alone |
| an agent's memory | an agent above 1.5 GB that uses at least twice the least of its last six samples is told in its chat; if that persists for four samples in a row it is stopped, with a notice |

A reading older than a minute says nothing about now, so a stalled sampler never blocks work. A level is announced when it
changes, not at every sample. To keep a reading that hovers at the limit from flipping every 10 seconds, the level falls
only 3 points below the threshold that raised it (critical until below 82 %, warning until below 67 %), and each chat has
**one** memory banner that is updated in place: a lower reading says it is easing, and coming back to normal turns the same
line into "back to normal". Older duplicate banners are coalesced when the host starts.

## Where it shows

| Client | Where |
|---|---|
| Desktop | Settings → Diagnostics → Computer health (numbers, agents, the two levels to edit); the Usage page's header (RAM, CPU, disk); a line in the notch island when memory is short |
| Android | the Status tab of the computer panel: level, memory, CPU, disk and agents; the Usage screen's header |
| iPhone | the Usage screen's header (RAM, CPU, disk) |
| Terminal | one amber word in the header when memory is short |

`hostHealth` is not local-only: a paired device with the Control scope reads it, so the owner sees a
joined or cloud computer's health from any of their devices through the relay. Codync accounts have
one owner and no member roles on a computer; a member of a network of computers cannot call another
computer's API, so they never see it.

## Memory already given back

Codync's own memory work predates this list and is measured, not simulated:

- **Idle agents.** A session unused for 30 minutes is closed; once every session is idle (or a
  routine run ends with no other open), the agent process stops with everything it started, and the
  next message relaunches and resumes it (`agent/bot/lifecycle.rs`). Under memory pressure the same
  release runs at once for every bot at rest.
- **Runaway agents.** An agent above 1.5 GB that keeps at least doubling its low-water mark is told,
  then stopped after four samples (`health.rs`), which frees the process group.
- **Browser tabs.** The Browser panel caps its tabs (`browser_panel::MAX_TABS`) and asks the user to
  close one; the agent browser is an isolated Chrome/Edge profile, so its tabs are the browser's
  memory, not a Codync window's.
- **One window per role.** The desktop app keeps one chat window, one pairing window and one island
  per display rather than a pool of hidden windows.

What is **not** done, and needs profiling before code: unloading idle *pages* in the agent browser
(CDP `Target.closeTarget` or `Page.setWebLifecycleState`), a window pool for the desktop app, and a
per-process private-memory reading on Windows. There is no measured win to claim yet, so no process
management was rewritten.

## Not done

- "Ask before an expensive turn when little memory is free": there is no measure of what makes a turn expensive. The limit
  refuses outright and the person can ask again.
- No SSE event: screens read `hostHealth` periodically (a new event type would be undecodable to older clients).
