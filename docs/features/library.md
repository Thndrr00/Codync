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

The method is host-side only for now: the clients still open a chat's Library. A server page in the
window bar (desktop), a screen beside a chat's artifacts (phones) and a terminal sheet can use it
as it stands, with the same empty, loading and error states as a chat's Library.
