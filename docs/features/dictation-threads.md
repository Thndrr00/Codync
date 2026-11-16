# Dictation threads

A dictation can be kept as a thread of its own instead of only filling the message box (OpenBot 0.60's
dictation threads). The thread is linked to the conversation it was dictated in, holds the transcript as its
first message, and the bot answers there, so a note you spoke while reading a chat becomes a side conversation
with the bot instead of a message in that chat.

## What the person sees

- **Desktop:** when a dictation has just filled the box, the composer shows **Keep as thread** beside the
  words. Choosing it creates the thread, sends the dictated words into it, clears them from the box, and opens
  the thread over the chat. The small `x` beside it drops the offer and leaves the words in the box.
- **iPhone:** the same action sits above the message box after a dictation, with the same result; the thread
  opens as its sheet.
- **Android and the terminal UI:** not in this pass. Android's app files are owned by another change this
  release; the terminal UI has no microphone and no dictation at all (see
  [voice-call.md](voice-call.md#dictation)).
- A thread that came from a dictation carries a microphone mark in the thread list and a **Kept from a
  dictation** line (with the message it was dictated under) at the top of the thread. On the desktop the line
  goes back to the chat.

The action appears only in a chat's own composer: not in a thread's box, not over an artifact, and not for a
bot that runs in the person's terminal. Words a voice shortcut sent are not offered (the message is already
gone). The offer lives until the next dictation starts, the draft's chat changes, or a message is sent.

## The data

A kept dictation is an ordinary thread (`threads`), so the thread list, archive, rename, unread counts and
deletion work unchanged. It is not a branch: its id is `thread:<uuid>`, never a message's id, so
`from_entry` stays empty. Two nullable columns were added to `threads` (schema 12):

| Column | Holds |
| --- | --- |
| `source` | `dictation` for a kept dictation; null for a thread the drawer opened. |
| `source_entry` | The chat message it was dictated under: the newest public user or agent message in the chat at the moment it was kept (or the `sourceEntryId` the app named). The chat itself and messages already in a thread are never linked. |

`ThreadInfo` now reports `source`, `sourceEntry` and `sourceText` (the linked message's text, read the same
way as `branchText`) beside `fromEntry`/`branchText`.

## Host methods

- **`keepDictation`** `{botId, text, sourceEntryId?}` → `{threadId, entry}`: creates the thread
  (`Store::create_dictation_thread`), titles it from the transcript (`dictation_title`: one line, at most
  `MAX_TITLE` characters, ending in `…` when cut) and posts the transcript as the user's message through
  `chat::messages::post_in_lane`, so the bot's agent gets it like any other message and answers in the thread.
  An empty transcript, a terminal-run bot, or an unknown bot is refused with its reason; nothing is created.
- The thread's lines come back through the existing `thread` method. No new event type: the created message
  and the thread row travel as ordinary entries and a thread summary.

## Tests

`host/src/store/threads.rs` covers the link: a kept dictation is its own thread, reports
`source = dictation` with the linked message's text, is not a branch, is not linked to another bot's message,
refuses an empty transcript, and cuts a long title at `MAX_TITLE`.
