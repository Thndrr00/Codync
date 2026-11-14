# Group chats and replies

Both features live in the host. Every client (iPhone, Android, desktop, terminal) calls the same
methods and renders the same data: none of them routes messages, parses mentions or builds
quotes itself.

## Words

- **Chat**: a bot's (or group's) one endless conversation.
- **Reply**: a user message that answers an earlier message of the same chat (the user's own
  or a bot's final reply). It stays in the chat, in time order, and carries a quote of the
  message it answers.
- **Group**: a roster row (`kind: "group"`) holding several bots and the user. It has a
  transcript but no agent, folder or harness of its own.
- **Lane**: where a turn talks: a bot's own chat or a group's. `host/src/store/model.rs` `Lane`.

## Data (all over the existing sync: `bot` and `entry` events)

- Bot: `kind` (`agent` | `group`), `members` (group: one or more distinct agent bot IDs on the
  same computer; no nested groups), `workingChat` (where the running turn talks). A group also has
  `title` (a short line under its name, `null` for none), `archived` (out of the open roster),
  `avatarPhoto` (the channel picture as a data URL: PNG, JPEG or WebP, at most 300 KB) and
  `keepMemories` (its members' memory writes in the room stay in the channel). A group's
  `status` / `activity` are its busy member's (`"Alice: Writing a reply…"`); its `lastMessage`
  names who spoke (`"Alice: …"`).
- Entry: `data.author` (the bot that wrote it; every agent-made entry has one); on a reply,
  `data.replyTo = {id, author, text}`: the answered message, who wrote it (`"user"` or a bot
  id) and its text (or its files' names when it has none), at most 500 bytes. The host builds
  it from `send`'s `replyTo` (`host/src/chat/replies.rs`).
- Read state: one per chat (`read_rev`). A bot's `unread` badge counts final replies and pending
  cards newer than it; a reacted-to message doesn't count again.
- The `events` catch-up carries each chat's recent entries; `history` pages older ones.

## Methods (`POST /api/<method>`, same on the E2E channel)

| Method | Body | Notes |
|---|---|---|
| `send` | `botId, text, clientNonce, replyTo?` | `botId` is a bot or a group. `replyTo`: the id of a user message or final reply in that chat; anything else is refused. Works in the relay mailbox too. |
| `createBot` | `{kind:"group", name, description?, title?, members, chiefBotId?, archived?, avatarPhoto?, keepMemories?}` | Returns the existing open group when these exact bots already share one. |
| `updateBot` | `{id, name?, description?, title?, members?, chiefBotId?, pinned?, hidden?, archived?, avatarPhoto?, keepMemories?, section?}` | A group can't become a bot or the reverse. `title`: a short line, `null` clears it. `archived`: out of the open roster, `false` restores. `avatarPhoto`: a data URL picture, `null` clears it. `keepMemories`: its members' memory writes in the room stay in the channel. `section`: a name, or `null` for none. |
| `deleteBot` | `botId` | A group's bots stay. A deleted bot leaves its groups. |
| `reorderBots` | `ids` | The roster as the user dragged it, top first; groups count as rows. See [bot order](computer-order.md#bot-order). |
| `renameSection` | `from, to` | Every row in section `from` moves to `to`, groups too; an existing name merges them, `null` removes the section. See [bot order](computer-order.md#bot-order). |
| `stop` | `botId` | For a group: ends its room turn, stops members working in it, and pauses its request tree (Continue below resumes it). |
| `react` | `entryId, emoji` | Toggles the user's reaction on a chat message (`data.reactions`, oldest first, at most 8). A reacted-to message never turns unread again. The bot isn't told, except in a group, where members see reactions on the room lines they're shown. Bots react themselves with the `chat` server's `react` ([bot reactions](chat-messages.md#bot-reactions)). |
| `respondPermission` | `entryId, optionId?` | Routed to `data.author`, so cards in a group reach the bot that asked. |
| `markRead` | `botId` | Reads the chat. A no-op when nothing there is unread, so clients call it whenever a chat is on screen. `send` marks its chat read. |

## A channel's title, picture and archive

A group is also a channel, and its editor carries the fields the agent tools use:

- **Title**: one short line under the name in the roster (`BotConfig::title`, at most 120
  characters). The roster shows it where the last message would go, and the apps' editors have a
  field for it.
- **Picture**: `avatarPhoto`, a data URL of a PNG, JPEG or WebP image at most 300 KB. The
  clients draw it instead of the members' cluster (`GroupAvatar` on iOS, Android and the desktop),
  the group editor offers Choose/Change/Remove Picture, and the terminal marks a group with a
  picture (`▣`) and can remove it. The same field is a bot's photo, so the validation in
  `host/src/hub/validate.rs` covers both.
- **Archived**: `archived: true` takes a group out of the open roster and stops its running turn; its
  conversation, members and files stay. `with_members` only reopens an open group, so making a channel with an archived
  group's exact members creates a new one. Every client's open list hides archived groups
  (desktop `BotStore.roster`, iOS `BotStore.roster`, Android `roster()`, TUI `App::roster`) and
  offers them where hidden bots live: **Archived groups** in the desktop's Settings, the iPhone's
  and Android's Computer management, and the terminal's `ARCHIVED` roster section (its editor has
  an Archived switch, so selecting one and pressing `e` restores it).

## An agent creates and changes channels (`host/src/chat/group/tools.rs`)

A bot whose settings turn on **Manage the team** gets two more tools in the `codync-manage` MCP
server (registered in `host/src/mcp.rs`, dispatched by `manageCall` in `host/src/api/bots.rs`):

| Tool | Fields | Notes |
|---|---|---|
| `create_channel` | `name`, `title`, `instructions`, `memberAgentIds`, `leadAgentId`, `archived`, `picture` | `instructions` is the group's `description`. The lead is the first member unless named. `picture` is a data URL or a PNG/JPEG/WebP file path (absolute or relative to the bot's folder), read into a data URL. An open channel with the same members is refused, naming it and `update_channel`. |
| `update_channel` | `channelId` (an id or a name), plus any of the create fields, `addMemberAgentIds`, `removeMemberAgentIds` | Only what is passed changes; `null` clears the title, lead or picture. Removing the lead clears it. A call naming no change is refused. |

Under Ask, both wait on the approval card `chat::manage::approve` uses for bots, listing the
channel's members, lead, title, archive state and picture (never the picture itself). Under Auto
the change applies at once. Either way a notice in the calling bot's chat says what happened
("Created channel Ship room", "Updated Ship room: title, members"). A call from a bot that cannot
see the channel (not a member, creator or lead) is refused with what to do ("ask the user to add
you to it, or to change it"); a bot without Manage the team is refused before anything else.
A created group reaches every client through the ordinary `bot` roster event, with its title,
members, lead and picture.

## A request's sub-tasks (`host/src/chat/group/tasks.rs`)

Every room turn is a small task tree, kept in the store (`group_tasks`, per group and lane): the
request (the user's message), an assignment for each member it asks, and children for the members
those assignments name. Each row has an owner, a state and the request's text (or the line that
brought it in):

| State | Meaning |
|---|---|
| `queued` | Waits its turn; a resumed branch is asked before anyone else. |
| `running` | Its member is taking the turn now. |
| `completed` | Its member took it (answered or passed). |
| `failed` | Its turn couldn't finish; it waits for Continue or Reassign. |
| `paused` | Stopped: nothing below this row runs until Continue or Reassign. |
| `cancelled` | Cancelled or dropped: neither this row nor anything below it runs again. |

The room turn checks the tree before every speaker (`tasks::next`), so a paused or cancelled
branch never speaks. A newer message in the lane supersedes the earlier request: its open rows
are cancelled, with the note "A newer request replaced it". A member that leaves the channel takes
its open rows with it.

Rows reach clients on the group's own `bot` JSON (`tasks`), so every client shows the same list
without another call. `groupTask` acts on one row:

| Method | Body | Notes |
|---|---|---|
| `groupTask` | `botId, taskId, action, ownerId?` | `stop` pauses the row and its branch (on the request: the whole tree, and the room turn ends); `continue` queues it and its paused children again; `reassign` gives it `ownerId` (a member) and resumes them with it; `cancel` ends the request from its root down, keeps the messages, and stops the members working there. Continue and Reassign start a resumed room turn when none runs, which asks only the rows that waited. |

The apps show the rows above the composer as **Waiting for sub-tasks**: owner, state and line,
with Stop on a row that waits or runs, and Continue, Reassign (a member picker) and Cancel on a
stopped one. A row's key on the terminal: `,`/`.` pick, `W` stop, `O` continue, `A` reassign,
`B` cancel (`groupTask` with the picked row).

## The lead drops a run (`host/src/chat/group/drop.rs`)

A person can ask the channel's lead to drop a request, for example when a newer one replaces it.
The lead has the `drop_request` tool in the `codync-chat` MCP server (registered in
`host/src/mcp.rs`):

| Tool | Fields | Notes |
|---|---|---|
| `drop_request` | `channelId?` | The channel's id or name; omitted, the channel the lead is working in. Only that channel's lead (`chiefBotId`) may call it. The newest request's whole run ends, with its running and queued sub-tasks, and the channel keeps its messages. |

Under Ask the call waits on the approval card `chat::manage::approve` uses for the team's changes;
under Auto it applies at once. Either way the channel gets a notice naming who asked ("The user
asked to drop this request: …") and the lead's lane a notice that it was dropped. `dropRequest`
(`botId`, optional `by`) does the same from a client, with `by` naming the person.

## A bot posts work into a channel (`host/src/chat/group/send.rs`)

A bot can also hand work to a channel from its own chat. Every bot has `send_to_channel` in the
`codync-chat` MCP server (registered in `host/src/mcp.rs`, dispatched by `chatCall` in
`host/src/api/bots.rs`):

| Tool | Fields | Notes |
|---|---|---|
| `send_to_channel` | `channel` (an id or a name), `request` (at most 8,000 characters), `members?` (ids or names) | Posts the request into the channel as the asking bot's message (`kind: "agent"`, `author`, `final`) and starts a room turn there. The named members answer it; with none, its lead routes it; with no lead, the first member. Returns the channel id. |

- It works only in the bot's own chat: in a channel turn it is refused, with the @-mention to use
  instead, so a room can't recurse into another room.
- The channel must be one the bot can see (a member, its creator or its lead), through the same
  `tools::find_channel` check `update_channel` uses; anything else is refused with what to do.
- Under Ask the post waits on the card `chat::manage::approve` uses (`Send to <channel>`, listing
  the members asked and the request); under Auto it applies at once. Either way a notice in the
  caller's chat says where the request went.
- The room derives the request from the entry: `since_last_request` starts at the last user
  message or channel request, `tasks::open` makes its task tree, and `requested` asks the names
  in `data.channelRequest.members` (or the lead). Later rounds and late answers work as usual.

## A channel's memory (`host/src/chat/memory/channel.rs`)

A group keeps a memory layer of its own, beside each member's own and the shared one
(`files::folder(Some(group))`). Its `MEMORY.md` is what every member reads in the room; a member's
own chats never see it.

| Tool (memory MCP server) | Fields | Notes |
|---|---|---|
| `channel_remember` | `text` (at most 500 characters) | Adds one line to the channel's `MEMORY.md`, or corrects the line with the same text (case aside) instead of adding a second. Refuses over the per-memory limit or when the layer's `MEMORY.md` is full, naming the limit. |
| `channel_forget_memory` | `text` | Removes the line with that text (exactly, else case aside); an unmatched text is refused with what to try. |

- Both work only in a turn of a channel the caller is a member of; the channel is
  `hub.runtime(bot).lane.chat`. Elsewhere they are refused with the actionable reason.
- Writes go through `files::write` with `Writer { channel: true }`: the usual checks still run
  (`check_change`), the "only in the user's own chat" rule doesn't apply, and the change is
  recorded against the channel layer, not the member's.
- A member's prompt gains the list (`channel::block`) before the room history, once per session
  and again when the channel's memory revision (`memory.revision.<group>`) changes.
- With `keepMemories` on, an ordinary `edit_memory` of the member's own layer made in a channel
  turn goes to the channel layer instead (`file_tools::edit`), so its other chats don't get it.
  `remember_for_all_bots` and shared memory are unaffected.
- `memory {botId}` accepts a group and returns its layer as `channel` (`yours` and `bot` empty),
  which the group settings show read-only on every client and in the terminal's memory sheet.

## How a group answers (`host/src/chat/group.rs`, `group/rooms.rs`, Grok Bot's design)

- The user's message starts a *room turn* in the group: up to 3 rounds, 10 replies in all.
- Who answers the first round: the members the user @-mentioned (full name, name without
  spaces, or first word; `@all` / `@everyone`) or replied to, or everyone if none is. A reply to
  a member's message counts like `@`-mentioning it.
- Typing `@` lists the group's members above the box, filtered by what follows it: each row shows
  the face, name and description, in the same list as the `/` commands
  ([marketplace](marketplace.md#commands)). On the desktop, with the caret at the end of the
  draft, ↑/↓ move the highlight and Tab or Return picks it, and Escape hides the list until the
  draft changes; on the phones a tap picks. A member's full name and its space end the list, so
  the next Return sends, and Return on a whole `@all` / `@everyone` sends, since the list doesn't
  offer them. The terminal has no `@` list: you type the name.
- Who answers a later round: only the members another member @-mentioned in this room turn
  (a member's `@all` / `@everyone`: all the others) and who haven't spoken or been asked since
  (one that passed after seeing the mention isn't asked again). Nobody named,
  and the room turn ends after the round: a member brings a teammate in by naming it, and a
  `(pass)` costs no extra rounds. Members speak one at a time; each round starts one member later.
- `message_bot` is refused in a room turn (its reply would go to the member's own chat); members
  @-mention each other instead ([bot collaboration](bot-collaboration.md)).
- A group's `description` ("About") tells its members what the room is for.
- A member runs the turn in its **own main session** (its history is unified across chats),
  told who's in the room and what was said since it last spoke there, framed as
  `[Group chat: "<name>" - with …]`. Its final reply is the one message the room sees; replying
  `(pass)` says nothing. A round where nobody speaks ends the room turn. A member can also
  react to someone's message (`react(emoji, to?)`), so it can agree with a reaction and pass.
  Room lines carry their reactions: `Ann: Shipped. (reactions: User 👍, Ben 🎉)`, and a reply
  its quote: `User: [Codync: this replies to Ann's message: "…"] text`.
- A member busy with a turn outside the room isn't waited for: the room shows "<name> is busy
  in another chat and will answer here when it's done.", moves on, and the member answers in
  the room once its other turn ends (with its own "done" push), on a prompt built then, so it
  sees what the room said meanwhile. It isn't asked again in later rounds of that room turn.
  Newer messages in the group, and Stop in the member's own chat, don't drop that late answer;
  the group's Stop or removing the member from the group does, and a newer room turn that asks
  the member again replaces it.
- A late answer (a busy member's, or one from a member the room stopped waiting for after 10
  minutes) that @-mentions teammates gets a *late room turn*: it asks the members named from that
  answer on who haven't spoken or been asked since, then goes on like any room turn. It starts
  right away when no room turn runs, or as the running one ends (a newer message of the user's
  drops it). Up to 3 late room turns follow one user message, and none after the group's Stop
  until the user writes again.
- A new message in the group, or Stop, ends a running room turn before its next speaker.
- Tool calls, thoughts and permission cards of a member's group turn are written to the
  group (with `author`), not to the member's own chat. One "done" push per room turn, plus
  one per late answer from a busy member.
- Files sent to a group are kept in the group's data folder (`~/.codync/bots/<group>/uploads`);
  each member's prompt lists their paths under the user's line.
- Voice calls work in a group: each final reply is read with its speaker's name, a member's
  pending permission card is announced (a group has no status of its own), and the realtime
  operator is told the room's members.

## How a reply reaches the agent (`host/src/chat/prompt.rs` `of_entry`)

In a bot's own chat a reply is an ordinary message in the same session. A note before its text
says which message it answers: `[Codync: this replies to your message: "…"]`, `… to the user's
earlier message: "…"]` or `… to Ann's message: "…"]`. It speaks as Codync, so the agent trusts the
quote even when the quoted message is no longer in its session. Memory never takes the
quote as the user's words: it stays in `data.replyTo`, apart from `data.text`.

## Client checklist

Roster row for groups (member avatars), a trace per turn, author label on group replies,
**Reply** on user messages and final replies (desktop: the hover footer beside the time, with
quick reactions and copy, never on the bubble, and the context menu; iPhone and Android:
long-press, reactions on top of the menu; terminal: `r` picks a message, ↵ or `r` replies, esc
cancels). While replying, the composer shows a quote
bar on top (`text.bubble`, “text” on one line, an X to cancel; the terminal: `↳ “…”`), its
placeholder reads "Reply…", and sending includes `replyTo`; leaving the chat cancels it. A
user message with `data.replyTo` shows the quote at the top of its bubble. Every client shows a
quote as its first non-empty line without Markdown markers (the roster's message preview rule). Reactions under
a message (the user's: tap to take back; the bots': emoji plus the avatars of who chose it),
group create/edit (name, title, picture, lead, archive, the **Keep memories in this channel** switch and the channel's read-only **Memory** list), Stop in a group calls `stop` with the group id and pauses its request tree. Every client's message menu also offers **Send to a channel**, which calls `forward` with the target chat or group (the desktop and Android had it; the iPhone gained it with `apps/ios/Kit/Sources/CodyncUI/Thread/ForwardSheet.swift`). Every client also shows **Waiting for sub-tasks** above the composer while a request has rows: each row's owner, state and line, Stop on a row that waits or runs, and Continue, Reassign and Cancel on a stopped one (host `groupTask`).

Slack-style reply threads were removed for quote replies; [threads for teams](../plans/threads-for-teams.md)
lists where their code lived and how to bring it back.

## A group's history settings

A group's settings have **Delete history**, **New conversation** and **Compact context**
([chat history actions](chat-history.md)). New conversation posts a divider and sets
`conversation.start.<group>`; a member's prompt reads the room only after it. Compact context queues
`Cmd::CompactNow` for every member, and because a member keeps one conversation across its chats, its
own chat is compacted too; the apps say so.
