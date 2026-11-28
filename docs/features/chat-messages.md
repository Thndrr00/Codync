# Chat messages

How a bot's work turns into what the user reads, copied from Grok Bot.

## Markdown links

Markdown links in bot replies are underlined on desktop and iPhone so they remain
recognizable in both light and dark themes. Hovering a desktop link shows its
destination URL; clicking opens it in the default browser. iPhone links keep the
system link interaction. The terminal already underlines Markdown link labels;
it has no pointer tooltip or link-opening action.

## Sound, links and Markdown in a reply

- An M4A, WAV, AAC, OGG, Opus, FLAC or MP3 attachment plays in the chat with the platform's own
  controls, on desktop and iPhone (OpenBot 0.70.0/0.72.0). The bytes come from the same store path
  as a picture; a sound a bot shared also downloads from the player. Android keeps these as file
  cards for now: it has no media3/ExoPlayer dependency, and its only `MediaPlayer` use is the
  voice-call port, not a reusable Compose player surface.
- A PDF or HTML file link in a reply opens in the chat's inert side preview instead of the system
  app (OpenBot 0.77.0/0.74.0): a sandboxed frame with no scripts or outside content, plus an **Open
  in system app** button. A local path of any other type, and a web link, still opens in its own app.
  On a Markdown surface outside a chat there is no preview to show, so the link keeps that fallback.
- A written path with folders opens only when all of its folders agree with a file the chat knows;
  a bare file name still matches by name, and a chain that matches nothing shows **File not found**
  rather than opening a different file with the same name (OpenBot 0.78.0). The pure decisions
  `match_file_link` and `file_link_target` live in `lib/chat-file-link.ts`; the Markdown link handler
  passes the chat's known files in through `FileCandidatesContext`.
- A target ending in `:line` or `:line:column` (`report.md:26`) opens the file at that place
  (OpenBot 0.78.0). Windows drive letters and folder names are kept, a malformed or absurd position
  is no target, and the in-app preview shows the named line next to its title and passes a `#line=`
  hash to the viewer. The sandboxed `file://` frame is foreign to the renderer, so it cannot scroll
  the document itself; a viewer that understands the hash can, and **Open in system app** keeps the
  same target. Scroll placement of new answers is unchanged (`lib/chat-answer-placement.ts`).
- A card posted during a turn stays at the row position of the entry it came from after it is
  answered and later messages arrive (OpenBot 0.78.0). `views/thread/card-position.ts` decides the
  order from the entries and is wired into `openQuestionCards`; it does not touch scrolling.
- A checklist written as `- [x] …` renders as a checked box and dimmed item text rather than a
  literal `[x]` (OpenBot 0.74.0). It is read-only: the box shows what the agent wrote and does not
  change the task. A picture that cannot be decoded shows **Image unavailable** and its file name.
- Wide tables, long code lines and diffs scroll sideways with an arrow and a fade on the side that
  still has content (OpenBot 0.72.0). The arrows follow the scroll position; narrow blocks show
  neither. Wrapped code stays wrapped and needs no arrow.

## Formulas in a reply

`$inline$` and `$$block$$` formulas keep exactly the characters the agent wrote, in a monospace
italic CSS class (OpenBot 0.70.0). The pure `math_blocks(text)` in `views/thread/markdown-blocks.ts`
splits literal text from math parts; `Markdown.tsx` renders them as escaped React text. There is no
KaTeX dependency, generated HTML, font download or network request, so a formula cannot run a script
or load anything. A missing close, empty content, a newline inside an inline formula, a dollar sign
inside one, and a price like `$5 and $10` (whose "content" ends in a space) stay plain text. A
`$$…$$` paragraph becomes its own block that scrolls with the reply; an unclosed one stays text.

The iPhone and Android renderers already show these characters literally — iOS
`AttributedString(markdown:)` and Android's CommonMark `inline()` treat `$` as ordinary text — which
is the required fallback. Neither has a sandboxed math surface: giving them one would mean adding a
WebView/KaTeX bundle (a network-capable HTML renderer) where the renderer is a native attributed
string, so the phone clients keep the text fallback rather than show half a formula.

## Visual replies and quote chips

An HTML artifact a bot publishes can be shown above its words as a visual reply (OpenBot `html_render`,
`agents.md`). It uses the chat's existing sandboxed preview (`views/thread/PagePreview.tsx` and
`lib/page-preview.ts`): an empty `sandbox`, no `allow-scripts`, and scripts, handlers, frames and
outside resources stripped, so the page cannot touch the app's storage or the network. The pure
`visual_reply(html, limit)` decision in `lib/visual-reply.ts` says whether an artifact is a page and
bounds it without leaving a half-written tag; the surface is client-side only and backed by the
artifact/file the chat already carries, with no new service, tool or artifact type. The frame keeps a
stable height so the messages below it do not jump, and a shortened page says so. The pure
`published_visual_reply(hasCard, html)` says when an agent's artifact card is that page;
`AgentBubble` in `views/thread/ChatRows.tsx` passes it and `onQuote` through `PagePreviewSheet` to
`VisualReply`, so the card stays above the words even while the tap-to-open sheet is closed.

Text selected on that surface can become a quote chip. `quote_text(selection, limit)` collapses
whitespace, bounds the passage and never makes an empty chip; the chip copies the quote when tapped.
An inert frame with an opaque origin keeps its own selection private, so where the browser does not
share it the chip falls back to text selected in the renderer-owned parts of the visual reply.
Reduced motion turns off the chip's fade. A selected passage is appended to that conversation's
draft by the row, just like quoting a whole message. Other clients keep their existing artifact
cards: their native chat surfaces have no sandboxed HTML preview to place above a reply, so this is
the desktop preview only, and nothing about the page or the quote is added to analytics.

## A page the agent needs you on

When an agent leaves a page in reach and asks for a sign-in or CAPTCHA step there, the composer shows
**The agent opened a page** with an **Open page** action (OpenBot 0.72.0). The pure `pageNotice` decision
in `lib/page-notice.ts` reads the latest agent entry's artifact card or web link and keeps the side
thread it belongs to, so opening the page from that thread goes back there. Desktop wires it in
`views/thread/Composer.tsx`; a user line, a tool row and an answer with no page show nothing, and the
notice fades out with reduced motion.

## Spoken requests

Internal harness instructions (`# AGENTS.md …`) that a live voice engine hears are dropped before the
words are posted to the chat, on desktop and both phones. The pure `spokenRequest` rule lives beside
each realtime engine (`views/call/voice.ts`, iOS `SpokenText.swift`, Android `Voice.kt`) and is checked
before the duplicate-utterance decision, so a re-sent transcript still posts only once.

## Messages are sent, not streamed

- Every bot gets the built-in `chat` MCP server (`codync-host mcp chat`,
  `host/src/chat/outbox.rs`) with `send_message {text}` and `send_file {path, name?}`. In the bot's own chat, each call is one chat bubble, added to the turn's lane at once as an `agent` entry with
  `final: true`. A turn can send several.
- The agent's own reply text (`agent_message_chunk`) stays in the trace when the turn sent
  text through `send_message`. A turn that sent nothing (an agent that ignores the tool, or has no MCP) falls back to
  the old rule: its last text becomes the turn's final message, unless that text is `(pass)`: a message that
  needs neither words nor a reaction gets nothing, as in a group. In a turn a steered message joined, what the agent
  wrote after its last `send_message` or reaction becomes a final message too, since either may have answered only
  what came before that message.
- File cards are separate final entries. Sharing a file preserves an intended final text
  reply; a files-only turn uses its filename for the completion summary. See
  [file attachments](file-attachments.md#files-from-a-bot). Artifact cards (`publish_artifact`) are final entries the
  same way; see [artifacts](artifacts.md).
- Group room turns, bot requests (`message_bot`) and routines reject `send_message` and `send_file`: their reply
  is the turn's last text, as before (`chat/group.rs`, `chat/team.rs`, `routines`).
- Between turns, work the agent goes on with in the background (a subagent's result) can still `send_message`
  (`send_between_turns`): a final entry in the bot's main lane, numbered with its latest turn, plus a *done* push;
  refused when the user stopped the last turn. `send_file` and `publish_artifact` need a running turn, and say to
  use `send_message` instead. The host can't tell a background call from the running turn's own (the `chat` MCP
  call names no session), so one that lands during a room, request or routine turn is refused like that turn's
  and told to send again once it ends.
- A bot request's reply stays in the recipient's trace and the request notice; it does
  not also become a message to the user. It goes back to the requesting bot as a turn of its own in
  that bot's main chat, and that turn's reply is an ordinary message to the user.
- The rules reach the agent twice: the MCP server's instructions, and the frozen instruction
  snapshot (`chat/context.rs`).
- The same server has `set_name {name}` and `set_instructions {instructions}` (`host/src/chat/profile.rs`): the
  bot changes its own name or instructions (its description) when the user asks it to, exactly as an edit in its
  settings would, and a notice ("Renamed to …", "Instructions updated") shows it in the main chat. A name set this
  way ends automatic naming. Its `save_skill` writes a skill for the bot, which the user can turn on for others
  ([skills bots write](marketplace.md)).
- The `chat` server also keeps the bot's working notes for one conversation: its main chat, or one thread
  (`scratchpad_read`, `scratchpad_write {text, mode: replace|append}`, `host/src/chat/scratchpad.rs`, kv
  `lane_key("scratchpad", bot, lane)`, so in a group each member has its own). Use them for a plan, what's done and what's next. The notes outlive
  compactions and session restarts. They are never in the transcript, memory or any client. They hold at most
  16,000 characters; an empty replace clears them, and they go with the bot, like other lane state. Idea from rakazo's
  `scratchpad-tools.ts` (Apache-2.0).
- Every change to a bot's instructions, from its settings, the API or `set_instructions`, is kept as a numbered
  version. Each version records the text, the author (`user`, `bot`, or `earlier` for what the bot had before its
  history started), an optional reason and the time (`host/src/chat/instruction_history.rs`, kv
  `instructions.history.<bot>`, removed with the bot). The last 50 versions are kept. `set_instructions` takes an optional `reason`
  and an `ifVersion`; a patch to `updateBot` can carry `instructionsReason` and `ifInstructionsVersion`. When the
  instructions moved past that version since, the edit is refused with a conflict instead of overwriting someone
  else's change. The bot reads its history with `instruction_history {version?}`; with a version, it also gets
  that version's text and a line diff from the one before. Clients get `instructionHistory {botId}`
  (`{current, versions: [{version, author, reason, at, chars}]}`), `instructionDiff {botId, from, to}`
  (`{diff}`, lines prefixed `- `, `+ ` or two spaces) and `restoreInstructions {botId, version}`, which saves
  that version's text as a new version. No client shows the history yet.
- A bot-to-bot exchange (`ask_bot`, `message_bot`) is one compact notice row, "Messaged Owen" /
  "Message from Egan". Its text lives only in the bot conversation sheet it opens; follow-ups
  are normal bubbles. Consecutive exchanges with the same peer in the main chat (either
  direction, trace entries ignored) collapse into one "3 messages with Owen" row ([bot collaboration](bot-collaboration.md#notice-data)).
- Clients never stream text into a bubble: the chat shows user messages, `final` agent entries,
  permission cards and notices (`isChat`). Nothing pops in and out while a turn runs.

## Pins

- `pinMessage {messageId, pinned}` on the `chat` server (`host/src/chat/pins.rs`) sets or clears
  `data.pinned: {by, at}` on a message; `pinnedMessages {botId, threadId?}` lists them. A pin rides the
  entry's rev, so one broadcast reaches every client; there is no pins table.
- `by` is `{kind: "person", device?}` for a person's pin (the device's name, or none at the computer
  itself) and `{kind: "agent", botId}` for the `pin_message` tool. The tool names the message by a quote
  because the agent doesn't see message ids, and it cannot take back a person's pin.
- Only messages are pinnable, 50 per chat or thread (the 51st drops the oldest), and a pin of a thread
  message carries the thread's title. The list is newest first across the bot's chats, 200 at most, and
  it answers `{entry, pin}` where `pin` reads `messageId`, `threadId`, `threadTitle`, `text` (first
  1 000 chars), `author`, `pinnedBy` and `pinnedAt`.
- Every client's message menu has Pin/Unpin, the chat header a pin button, and a
  client that shows the chat and its threads shows both sheets: the pin opens its message, a thread at
  that message and a button back to the latest one. The terminal uses `B` for the list and `p` to
  pin/unpin the picked message, and marks a pinned row with `· Pinned`.
- The agent is told about pins only through its tool; a pin by a person reaches the agent's prompt on
  the next turn as the message text it already reads.
- A pin or an unpin counts in analytics as `message_pinned` (`pinned`: which way it went), never which
  message or its text ([analytics events](../reference/analytics-events.md)).

## The chat header

The header names the agent and, after the name, its purpose (`description`), so two agents that share a
name are told apart: `Brock · Store manager`. The purpose leaves first as the header narrows — a
container query on the desktop pill, `ViewThatFits` on the iPhone, a width check on the Android header,
and no purpose at all in the terminal's narrow layout — so the name keeps its room. Ported from OpenBot
0.70.0, which added the agent's title to the chat header.

## Message actions

A message row's menu has **Copy link**, the stable `bot:<id>/entry:<id>` string the host also answers
as `messageLink {botId, entryId}`; an image attachment can be copied from its right-click menu
through the same store bytes and clipboard helper as the viewer's Copy. Hovering a reaction chip
names everyone who chose that emoji (you and the bots in the store). **Compact message actions** is
a device-only preference that hides the per-message time under each bubble. A working bot's status
row comes from the live mirror, so a history search reads **Searching its past chats…** as soon as
the host publishes it. A steer or queue call that mistypes a bot id names the close bot instead of
just saying the id is unknown.

## Several people

- The people of a chat are the devices paired with the computer (`host/src/chat/people.rs`): each by its
  own name and a colour picked from its key, so two people on one computer are told apart. The computer
  itself is the owner and has no author.
- `presence {botId, typing?}` says the caller is looking at the chat and answers `{people, at}` with the
  other devices there, each `{key, name, color, typing}`. It is in memory only: a device drops out 30 s
  after it last looked, and counts as typing for 4 s after its last key (`typing: false` ends it sooner).
- A user message sent by a device carries `data.person: {key, name, color}` (`api/bots.rs`'s `send`);
  written at the computer it has none. It still counts as the owner's message everywhere: no new
  permissions, notifications or push.
- Every client polls `presence` every 3 s while a chat is open, sends its own `typing: true` at most once
  a second while a key comes (3 s locally, cleared on send) and shows the others above the message box:
  initials in their colour and "Ana is typing…" / "Ana is here". A run of a device's messages carries
  its name once, and runs group by device, mobile included; the terminal names the person on the user
  line instead of "you".
- The first presence with somebody else in a chat sends `presence_shown` (`others`, `typing`), once per
  chat per host run ([analytics events](../reference/analytics-events.md)).

## Bot reactions

- The `chat` server's `react {emoji, to?}` (`host/src/chat/reactions.rs`) goes through the bot's actor,
  which puts one emoji on the message the turn answers: in its own chat, the user's message that
  started the turn; in a group, the newest message its prompt showed from `to` ("user" or a participant's
  name), or from anyone else. Routines and bot requests can't react. Each bot keeps one reaction per
  message; a new emoji replaces it.
- Stored as `data.botReactions: [{bot, emoji}]` beside the user's own `data.reactions`. The first one also
  records `data.contentRev`, the rev before it, which the unread count uses, so a read message stays read.
- Reactions are rare. The instructions allow them only where a person would react (thanks, a plain OK, big
  news, a real joke), never on a question or a request, and the host refuses a bot's reaction while it
  reacted to another of the chat's last 6 messages (`reacted_lately`); a turn whose only answer was that
  refused reaction says nothing and sends no push. A reaction can be the whole reply: a turn that reacted and sent no message
  doesn't fall back to its last text, and its push says "Reacted ❤️ to your message".
- Clients show each bot emoji as a chip with the avatars of the bots that chose it (up to three), after the
  user's own chips; it isn't tappable, and a bot that was deleted drops out. The terminal UI lists
  `❤️ Ann, Ben` after the user's reactions.

## While it works

The working line under the last message shows the bot's activity; composing a `send_message`
reads as **Typing…** (`activity()` in `agent/bot/tool_entry.rs`). The activity indicator shows an
animated orb, a short status and elapsed time, in a pill no wider than an agent bubble whose status
truncates to one line. Replies arrive as complete messages.

**Show thinking details** (Settings › Chat; each device's own choice, off by default) decides whether the pill
unfolds; off, it stays a plain status line. On, the pill ends in a chevron, and a tap (click) unfolds the running
turn's steps inside it:
the turn's thinking text, narration (never a `(pass)`), tool calls (MCP tools included) and plan, in order, with each
tool's output and diffs one more tap away. The pill widens to an agent bubble's width, the steps
scroll inside it past 280 pt and follow new ones until the reader scrolls up, and each turn starts
folded; the pill leaves with the turn. The host sets `bookkeeping` on every tool entry: `true` for
the bot's own bookkeeping (a `send_message` call, a `memory` MCP call, or a call naming a file in a
memory folder the bot sees), which the steps leave out, as the working line does. A host older than
2.25.0 sets none, so its tool calls stay out of the steps too. A turn with no steps yet unfolds to
**Nothing yet.**: some agents send no thinking text, and Claude's arrives only as its thinking ends.
Claude Code leaves an SDK session's thinking text out unless asked, so Claude bots start with
summaries shown (`--thinking-display summarized`, through `claudeCode.options.extraArgs` in
`agent/bot/launch.rs`). Whether Claude thinks stays the user's Claude Code setting; when it does, a
short thought's summary arrives in one piece when the thinking ends, a long one's in parts.

Full conversation remains available through a message's Show what it did (its touch-and-hold
menu on the iPhone and Android, its actions on the desktop). It contains narration, all
thoughts, tools and previous turns, and follows new steps until the reader scrolls up.
On the terminal, with thinking details turned on (`^k` › Show thinking details, kept in
`tui-thinking-details`), `>` or a click on the working line unfolds the steps under it (their last
14 lines), `t` opens Full conversation, and the working line keeps the count of steps so far;
once the turn ends, its last message's `↳ N steps` line counts them the same way.

## Motion and scrolling

- A message arriving while the list follows the newest pops in: 0.24 s,
  `cubic-bezier(.23, 1, .32, 1)`, from 12 pt lower at 94 % scale (iPhone: `ConversationLayout` in
  `ConversationList.swift`; desktop: `.row-in` in `thread.css`; reduced motion: a 0.12 s fade).
- The list stays pinned to the bottom while the reader is there; scrolling up releases it and the
  round "Jump to latest" button brings it back. Sending always returns to the bottom.
- Opening or closing the phone keyboard keeps a bottom-following chat on its latest message;
  reading older messages preserves that position. iPhone viewport updates do not count as user
  scrolling and wait for any active touch to finish before applying a deferred bottom follow.
  Android already follows viewport/padding changes; desktop observes transcript resizing, and
  the terminal renders its bottom-relative scroll position without a software keyboard.
- iOS 26+ uses the native circular Liquid Glass button for "Jump to latest", with a material
  fallback on older iOS. Android and Electron retain their platform controls; neither exposes
  Apple's native Liquid Glass button API. The terminal keeps its keyboard action.
- iPhone: earlier messages load when the top of the chat comes into view, so a short chat never
  shows a lasting spinner. Pulling down past the top sweeps in the `arrow.clockwise` symbol; letting
  go of a full sweep (`PullToRefresh.swift`) reconnects to the computer while the chat waits just
  below the turning arrow, then eases back. A pull that isn't full refreshes nothing. The desktop
  reconnects from the computer menu instead.
- Android: the phone stores each chat on disk and holds only a window of it in memory: the newest
  300 messages of the open chat and the 5 opened last, the newest 30 rows of every other chat.
  Earlier messages load once the top of the chat is on screen: rows the phone already stored come
  first, then a page from the computer, and the window grows by 200 messages each time. It stops at
  3,000 messages or a per-chat memory budget (tool-heavy chats sooner); the iPhone pages to the
  first message. `apps/maestro/history.yaml` scrolls a long chat back to its first message on both phones.
- The terminal UI lists the same messages; its steps line goes under the turn's last message.

- Electron keeps the activity row mounted during its 0.42 s exit and animates its height,
  opacity and a small scale change together. The transition honors Reduce Motion and reverses
  when work restarts during an exit. iPhone conversation insertion, removal and scrolling
  keep their existing animations.

## Stored messages

- One undecodable transcript row no longer makes a chat unreadable. The entry loaders read through
  a projection that substitutes a JSON `null` for a malformed cell, so SQLite's `json_*`
  predicates don't abort the list; `skip_corrupt` then drops just that row from the result while
  it stays on disk and a warning names only its id and kind (OpenBot triage-15865). Every other
  message of the chat, and every other chat, still opens.
- Deleting an agent removes its own lines from the channels it shared with other members — the
  entries it wrote (`author`) and bot messages it sent to or received from another bot
  (`sourceBotId` / `targetBotId`) in another chat's lane. Other members' channel messages stay,
  as do the deleted bot's own chat entries that `deleteBot` already cleared (OpenBot
  triage-15916). `own_channel_entry` in `host/src/store/entries.rs` is the pure decision, and
  the purge runs just after the bot row is deleted in `host/src/hub/bots.rs`.
- **Font lists:** reopening Settings › Appearance shows the last installed-font list immediately
  from a device pref while a fresh read replaces it (OpenBot 0.80.0).
