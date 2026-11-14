# Library

A chat's Library lists what its bot published and what was shared in it: artifacts, files a bot
sent (`send_file`) and files the user attached. A server's Library lists the same for every bot of
one computer, newest first. Nothing on the computer's disk is scanned: both read the host's own
stores.

## A chat's Library

`library {botId}` → `{items: [{kind: "artifact" | "file" | "attachment", at, entryId?, artifact?,
file?}]}` (`host/src/chat/library.rs`). Artifacts are the bot's public ones, without the internal
bookkeeping entries. Files come from the newest 500 messages that carry any. The apps group the
items by when they landed (Today, Yesterday, This week, Earlier), newest first, and open each the
way the chat does: an artifact with `readArtifact`, a bot's file with `readFile`, an attachment
with `readUpload`.

- iPhone: `LibraryView` (`apps/ios/Kit/Sources/CodyncUI/Artifacts/LibraryView.swift`), opened from
  the chat's artifacts card.
- Android: the library sheet (`apps/android/app/.../LibrarySheet.kt`).
- Desktop: the artifacts/library view (`apps/desktop/src/renderer/views/library/`).
- Terminal: `Ctrl+K` → **Library of this bot: artifacts and files…** (`L`).

## A server's Library

`serverLibrary {filter?}` → `{items: […]}` with the same item shapes plus `botId`, `botName` and
`image` (true for the extensions the apps preview: png, jpg, jpeg, heic, gif, webp, tiff, bmp).
`filter` is `images` or `files`; anything else shows everything. Items are deduplicated by kind
and id (a file forwarded to two bots shows once), sorted newest first, and a bot that was deleted
is left out. The list is built from `library` for each bot, so the per-chat limits apply.

The clients show it as one page, with a group a computer on the desktop, newest first by day, and
an **All / Images / Files** filter. The page only exists on a host from 2.48.0 (`hasServerLibrary`
on each client).

- iPhone: `ServerLibraryView` (`apps/ios/Kit/Sources/CodyncUI/Artifacts/ServerLibraryView.swift`),
  opened from the book button beside the roster's filters or from a chat's Library page; its rows
  use the chat's viewers and share sheet, and a computer picker appears when several are shown.
- Android: `ServerLibrarySheet` (`apps/android/app/.../ServerLibrarySheet.kt`), opened from
  **Library** among the roster's actions or from a chat's Library page.
- Desktop: `ServerLibraryView` (`apps/desktop/src/renderer/views/library/ServerLibraryView.tsx`),
  a sheet opened from the window bar's **Library** icon or the search palette. It reads every
  connected computer and groups each one's rows by day.
- Terminal: `Ctrl+K` → **Library of every bot: artifacts and files…**
  (`host/src/tui/manage/server_library.rs`, `host/src/tui/sheets/server_library_view.rs`);
  `Tab` filters, `Enter` opens an artifact or a file's text preview, `f` saves, `r` renames.

Each client filters locally after one read (`images`/`files` are sent by the host too), so
switching the filter never waits for the computer.

