# People on a computer

Codync has one computer owner and many paired devices, but no member sign-in. A **person** is the
record that stands in for a member account: someone who uses this computer with their own paired
phone, or with bots of their own. The owner is implicit; a device, a bot or a connector with no
person belongs to the owner.

This is the security model for member-scoped MCP servers: one person's connectors, OAuth tokens and
chats are never visible to another person's phone, only in the host that authorizes the call.

## What a person is

- A row in `people` (`host/src/store/people.rs`): a name, an optional email and when it was added.
- A **device** can be assigned to a person (`devices.person_id`). That phone then acts as that person:
  the host resolves `Caller::Device { key }` to the device's `person_id` and authorizes every call by
  it (`people::caller_person`). A device with no person is the owner's own.
- A **bot** can belong to a person (`BotConfig.person`). A bot created from a member's phone defaults
  to that person; the owner can pick one in the bot editor. Empty means the owner's.
- A **connector** is shared or personal (`Connector.person`). Empty is shared with every bot on the
  computer. A person's connector is seen only by that person's bots and devices.

There is no member login, and this change adds none: the owner creates people and assigns devices and
bots on the computer itself. Those methods (`people`, `addPerson`, `updatePerson`, `removePerson`,
`setDevicePerson`) are loopback-only, so only this computer's own app can call them. A member's phone
cannot list other people, and cannot assign anything.

## What a member's phone sees

A device assigned to a person gets the same calls as the owner's phone, but scoped in the host:

- **Roster and chats.** Only that person's bots and groups. The event stream filters bots and entries
  before they leave the host (`api::events`), and every method that names a bot, a message or a thread
  refuses another person's (`people::check_bot`, once per call in `api::dispatch`).
- **Cross-chat search.** A bot's `search_history`, `read_history` and memory source lookups read its
  own chat and the groups of its person (`chat::memory::chats`). They never read the owner's chats.
- **Connectors.** `connectors` lists shared connectors plus that person's. A member's install is
  theirs even if the request names someone else; they cannot update, sign in to or remove a shared
  connector or another person's (`market::owned_by`).
- **Bots.** `createBot` from a member sets their person; `updateBot` keeps it. A member cannot move a
  bot to the owner or to another person.
- **Cross-people guard.** A member's `updateBot`, `deleteBot`, `send`, `history`, `thread`,
  permission answers and the rest all pass the same bot check, so guessing an id gains nothing.

The owner and this computer still see everything: that is what owning the computer means. Removing a
person is how their access ends; their bots and chats stay (see below).

## Shared and personal connectors

| Where | Owner (this computer, or a device with no person) | Member's device |
| --- | --- | --- |
| `connectors` list | every connector | shared + their own |
| Add a connector | shared, unless a person is chosen (no client offers that yet) | always their own |
| Update, sign in, remove | shared ones | only their own |
| A bot's MCP servers | shared connectors | shared + that bot's person's |
| Composio apps | computer-wide; only the owner connects them | visible, read-only |

A personal connector keeps its own settings and its own sign-in: ids are unique across the computer,
so two people connecting the same app get separate entries with separate credentials in the same
vault. Nothing about one person's token reaches another person's bot, because the host leaves the
connector out of the servers it gives that bot (`agent::bot::session`), and out of the tool list the
bot's `connectors` server returns (`market::requests`).

## Removing a person

**Settings → People → Remove** asks first, then unassigns everything instead of deleting it:

- Their devices become the owner's (they keep working as the owner's phone, and the owner may assign
  them again).
- Their bots and chats become the owner's; no bot or conversation is deleted.
- Their connectors become shared, so every bot on the computer can use them.

The confirmation says so before the call. Removing a person never deletes a connector, a bot or a
chat, and never revokes a device by itself: use **Revoke access** in the device list for that.

## Clients

- **Desktop:** Settings → People: add a person, rename or change their email, remove one, and assign a
  paired device from the row. Extensions shows a person badge on each connector (or **Shared**), and
  a member's view says **Shared** and **Mine**.
- **iPhone and Android:** Computer management → People, with the same actions, for the owner's phone.
  The section is hidden when the host refuses it, which is what a member's phone gets.
- **Terminal UI:** the host methods are the same ones the other apps call; its own People page is not
  built yet, so people are managed from the desktop app or a phone today.

The desktop app and the phones talk to the same host methods; every check is in the host, never in a
client.

## Limits

- No member passwords, accounts or invitations exist. A person is a label the owner puts on a device
  or a bot; whoever holds the paired phone acts as that person.
- The owner can read every member's chats and use their connectors. The reverse is refused.
- Composio connections and skills are computer-wide: a member sees them and can use them in a bot,
  but only the owner connects or removes a Composio app.
- A member's own new bot gets the shared connectors plus their own. It never gets another person's.
