# Saved searches, quick actions and routine run history

These interface additions target Windows and Android. The iPhone, terminal, macOS and Linux
interfaces retain their existing controls. The desktop remains one shared Electron codebase;
Windows-only controls use its platform check.

Openstark was the UX reference for saved queries, searchable actions and run inspection.
Codync adapts these ideas to its React desktop app, Android Compose app and shared Rust host.
It uses Codync's existing controls, navigation callbacks and host API; no SolidJS UI was copied.

## Message search and saved shortcuts

On Windows, open Search and select **History** to search messages from the visible roster across
its computers. Results retain their bot and thread context. Select a result to load its context
and continue in the corresponding chat or thread. Successful computers retain their results when
another computer fails; a warning identifies missing results and offers retry.

On Android, open **Search conversation** to search the selected bot's main chat and threads.
Search results come from the host, support pagination and open the matching message context.
Search and pagination errors remain visible with a retry action.

Both interfaces offer these message scopes when the host supports them:

- `all`: main chats and threads.
- `main`: main-chat messages only.
- `threads`: messages in threads only.

Save a nonempty query with its scope, restore it from the saved list or remove it. A shortcut saves
the query and scope, not the result messages or a conversation selection. Windows lets the user
choose which computer holds the saved list; its history search still covers the visible roster.
Android applies a restored query to the currently open conversation.

The host owns validation and persistence. It accepts at most 20 shortcuts per authenticated
device on that host. A query must contain at most 256 characters and eight whitespace-separated
terms. Saving trims surrounding whitespace; the same query and scope, ignoring query case, do
not create another shortcut. At the limit, remove a shortcut before saving another.

`savedSearches`, `saveSearch` and `deleteSavedSearch` use the authenticated caller's device key.
Preferences are stored in the host's SQLite key-value table under `saved-searches.v1:<device>`.
Authenticated local loopback clients share that host's `local` device bucket.
They survive app restarts and are independent for other devices and other computers; signing in
to the same account does not synchronize these lists. Clients mirror the host response rather
than maintain a separate durable copy.

Hosts without these methods still support ordinary **All messages** search. Saved shortcuts and
scope filters show as unavailable with retry, rather than claim to filter an unsupported host.
A scoped Windows search also checks each computer's support, so an old computer cannot silently
return unfiltered results labelled as a narrower scope.

## Quick navigation actions

Windows Search has an **Actions** mode with searchable entries for Search history, New bot,
New group, Settings, Computers and Account. Creation entries are available when a computer is online.

Android places an expandable **Actions** entry below the roster controls when the query is empty.
Typing matches New bot, New group, Computers and Account by their labels. Creation entries are
omitted while offline; Computers and Account remain available. These actions call the existing
navigation and creation callbacks and do not implement a second creation flow.

## Routine run inspector

Expand **Run history** beneath a routine to browse its recorded runs. The list shows up to ten
recent rows. Select a row and use Older/Newer to browse every run for that routine included in the
host response. The host currently returns up to 200 recent runs for the selected bot; this view
does not claim to expose older runs outside that response. An empty history has an explicit
message.

The inspector displays recorded status, creation time, completion time when present, detail when
present and the scheduled retry time (`notBefore`) when present. It does not reconstruct an old
instruction from the routine's current settings or invent output snapshots. Browsing history has
no execution side effects. Existing run-now, enable/disable and chat-notice retry actions retain
their existing behavior.

## Implementation entry points

- Host: `host/src/api/saved_searches.rs`, `host/src/store/saved_searches.rs` and the existing
  `searchMessages` and `routines` operations.
- Windows: `views/SearchPalette.tsx`, `views/SavedHistorySearches.tsx`, `views/SearchActions.tsx`,
  `store/search-shortcuts.ts`, `store/roster-query.ts` and `views/routines/RoutineRunHistory.tsx`
  beneath `apps/desktop/src/renderer/`.
- Android: `ConversationSearch.kt`, `ConversationSearchModel.kt`, `AppStoreSearch.kt`,
  `RosterQuickActions.kt` and `RoutineRunHistory.kt` beneath
  `apps/android/app/src/main/kotlin/com/codync/android/`.
