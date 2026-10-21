# Roster: search and actions

The roster retains Codync's avatars, surfaces and synchronized manual order.
Filters, metadata search and project grouping only change presentation. Reordering
is available with an unfiltered roster in manual order, so a filtered subset
cannot overwrite the host's order.

## Block C (ideas 17–24)

- **17:** metadata search matches name, description, declared workspace path and
  agent. History is a separate mode backed by the host, with pagination and a
  bounded context window for the selected main-chat or thread message. It never
  inserts disconnected history windows into the continuous mirror.
- **18:** All, Unread, Needs attention, Working and Pinned select independent
  presentation filters. Needs attention includes errors and requests for input;
  it does not mean a permission can be approved. Android List and Constellation
  use the same selected bots; the latter labels a filtered figure.
- **19:** horizontal gestures reveal explicit actions. Stop requires a button
  press. Context menus and keyboard controls remain available. Actions use the
  existing host commands; failures retain confirmed host state.
- **20:** the attention filter is the attention panel. Opening a bot shows its
  existing permission cards, with their actual entry IDs, context and options.
  The summary deliberately routes to detail instead of approving from a status
  flag or incomplete cached information.
- **21:** rows show agent, declared project and requested/available/refused Fast
  state. Group chats never borrow a member's workspace or Fast state.
- **22:** sections preserve the host's order within each declared workspace.
  Keys include device context and computer, plus the full declared path. Clients
  do not canonicalize remote paths. Folding is a device preference; group chats
  and bots without a project have their own sections.
- **23:** durations use the host's turn `startedAt`, not creation or last-message
  time. A missing start stays unavailable. Offline state does not advertise a
  running clock. Stop remains pending until the host changes the turn/status;
  missing confirmation is reported instead of setting idle locally.
- **24:** previews request the last three main-chat user/final-agent messages.
  They do not select a chat or mark it read. Closing or changing context retires
  the read. Android supports this in both roster presentations, iOS in the
  native context menu, desktop through hold/Space and an explicit Preview action,
  and the terminal through its roster tools.

Android touch gestures adapt to mobile input. Desktop uses the same renderer on
macOS, Linux and Windows. iOS keeps native context menus and list navigation.
The terminal uses keyboard equivalents instead of touch gestures.

## Terminal controls

`Ctrl+f` opens roster tools. Type to filter metadata; `Tab` cycles the status
filter. `Ctrl+h` switches to history; Enter starts a history query and opens a
selected result. `Ctrl+n` loads another page. `Ctrl+p` previews the highlighted
bot without marking read. `Ctrl+g` toggles project grouping and `Ctrl+b` folds
the selected bot's project. `Ctrl+l` clears metadata/status filters. Escape
closes tools. Existing pin, hide, reorder and stop shortcuts remain; reorder is
disabled while filtered or grouped.

## Read API

All three methods pass through the existing authenticated dispatcher. Loopback
requires the bearer token; encrypted remote devices require Control permission.
They operate only on that host's visible, non-deleted bots. Pairing callers and
devices without Control cannot use them. No account ID supplied in a request
can expand authorization.

`searchMessages` takes `query` (1–256 characters, up to eight whitespace-separated
terms), `botIds` (a nonempty list within the transport request-size limit),
optional exclusive `beforeSeq`, and `limit` (default
30, clamped to 1–50). All terms must occur in a user message or final agent reply.
Unicode lowercase is applied in the host, shared with its existing memory history
search. `%` and `_` are literal characters, not wildcards. Results include `id`,
`botId`, `threadId`, `seq`, `createdAt` and a 400-character text excerpt. `next`
is the last returned sequence when another page exists. Reuse the same query
and bot scope with that exclusive cursor. New messages belong to a refreshed
query; edits/deletions can change later pages.

`rosterPreview` takes `botId` and returns `entries`: up to three visible main-chat
messages, in chronological order. It never advances read cursors.

`messageContext` takes `botId` and `entryId`. It validates that the entry belongs
to the requested visible bot and returns up to 20 entries through the match,
20 following entries in its lane, plus the thread root when applicable. Clients
render user/final-agent messages from this separate window and focus the match.

Invalid requests and unavailable bots/messages return an error; authorization
failures are distinct. Older hosts return an unsupported-method error, which
clients display as a failure rather than an empty successful search. There is
no offline full-history fallback. Metadata filters remain available offline.
Reads run off the async runtime on the existing SQLite connection. They have
bounded result counts and do not mutate revisions or read state. Closing or
replacing a client query cancels its owner and rejects late results; an already
running SQLite read may finish on the host. Reads are retryable; search requests
are debounced on graphical clients. These are additive methods and do not raise
client/host compatibility minimums.

## Acceptance checks

Check metadata filters and project folding without changing synchronized order;
repeat with keyboard or screen-reader actions. In Android, compare List and
Constellation for the same filter. Check distinct full paths with identical
folder names and the separate group-chat section.

Search history, replace the query while loading, page results, and open a
thread match. Closing the search or switching accounts must retire outstanding
opens. Preview a bot without changing its unread count; closing the preview
must retire its read. Missing history support and transport errors must show
errors rather than successful empty results.

Stop a working bot, including while disconnecting. Only a confirmed turn/status
change clears the pending action successfully; missing confirmation must show
an error. Reconnect and confirm that duration still uses the same host turn
start. Desktop's shared clock subscribes only visible, online rows.

Automated roster coverage lives in `host/src/store/roster.rs`,
`host/src/tui/app/roster_tools.rs`, `host/tests/roster_e2e.rs`, desktop's
`roster-query.test.ts` / `roster-requests.test.ts`, and Android's
`RosterQueryTest.kt` / `RosterControlsTest.kt`. Passing these checks does not
replace the native-app acceptance flows above.
