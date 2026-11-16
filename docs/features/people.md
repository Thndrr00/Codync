# People and member accounts on a computer

Codync has one computer owner and many people who use it. The owner is implicit; everyone else is a
**person** with a role, and a person can have an **account**: an invited email, a one-time invitation
and a session that proves they are that person. A device, a bot or a connector with no person belongs
to the owner.

This is the security model for member-scoped MCP servers: one person's connectors, OAuth tokens and
chats are never visible to another person's app, only in the host that authorizes the call.

## What a person is

- A row in `people` (`host/src/store/people.rs`): a name, an optional email, when it was added and a
  **role**: `member` (their own bots, chats and connectors) or `admin` (they also manage everyone's
  people, connectors and bots). The owner is implicit and is not a row.
- A **device** can be assigned to a person (`devices.person_id`). That phone then acts as that person:
  the host resolves `Caller::Device { key }` to the device's `person_id` and authorizes every call by
  it (`people::caller_person`). A device with no person is the owner's own.
- A **bot** can belong to a person (`BotConfig.person`). A bot created from a member's phone defaults
  to that person; the owner can pick one in the bot editor. Empty means the owner's.
- A **connector** is shared or personal (`Connector.person`). Empty is shared with every bot on the
  computer. A person's connector is seen only by that person's bots and devices.

## Inviting a member

A member account has no password. The owner, or an admin, hands out an **invitation** from People:
`inviteMember` takes an email (and an optional name and role) and answers with the person and a
one-time code. `docs` and the table only ever hold the code's digest (`member_invites`,
`store/member_accounts.rs`), and inviting the same email again replaces the earlier code. An
invitation lives for seven days.

The code is also a pairing code: it carries the computer's address and keys, so the member's phone
scans it with the same "Pair a computer" screen and the host files that device under the person with
**Control** scope (no screen). A member at the computer itself (the terminal UI, or the desktop app
over its own tunnel) redeems the code with `acceptMemberInvite`, which answers with a session token
instead of pairing a phone.

Both paths end in one place: a `devices` row whose `person_id` is the member, authorized by
`api::devices::permit` and narrowed by `people::caller_person`. There is no second authorization
system, and a member's invitation never grants a role of its own: the role comes from `people`.

Only the owner can invite an admin, or change anyone's role. An admin can invite members and end
members' sessions; another admin's invitation and session stay the owner's.

## Sessions

A **session** is a way of acting as a person until it is revoked:

| Kind | How it starts | Ends |
| --- | --- | --- |
| Paired device (`kind: "device"`) | The member scans the invitation QR | `revokeMemberSession`, `revokeDevice`, or removing the person |
| Signed-in session (`kind: "session"`) | `acceptMemberInvite` at the computer, or `memberSignIn` | `revokeMemberSession`, or signing out |
| Invitation (`kind: "invite"`) | `inviteMember` | Redemption, expiry, `revokeMemberSession`, or removing the person |

`memberSessions` lists them for the owner (and, for an admin, the members and their own session):
the person, the device or invitation, when it started, when it ends, and whether it is connected.
`revokeMemberSession` takes the session id: a device loses its row, push tickets and token at once; an
invitation stops working. The person, their bots, chats and connectors stay, and they can be invited
again.

A signed-in session's token is checked on this computer's local API only: the desktop app, or any
client that reaches that API over its own tunnel. Requests from the network authenticate through the
E2E channel instead, which is what the paired path uses. Tokens expire after 30 days.

## Company sign-in

Settings → Company sign-in configures an OpenID Connect provider and the email domains it owns. A
verified email at one of those domains maps to a person with `enterprise::member_from_email`, and the
login exchange `memberSignIn` turns it into a session at the computer. It never creates a person or a
role: the email has to have been invited already, and the domain has to be one the owner named. The
exchange is a loopback call, made by the desktop app after the provider verified the email; it trusts
a client that already holds this computer's token, exactly as that token's owner does.

SCIM provisioning (`/scim/v2/Users`) now writes to `people`: creating a user creates a member person,
updating one renames them, and deleting one removes them exactly as Settings does. It never grants an
admin.

## Admins and members

| | Member | Admin |
| --- | --- | --- |
| Bots and chats | Their own | Everyone's |
| Connectors | Shared and their own; changes only their own | Everyone's; what they add is shared |
| People and devices | No | Yes, from their phone (`people`, `addPerson`, `updatePerson`, `removePerson`, `setDevicePerson`, `devices`) |
| Invitations and sessions | No | Members' (`inviteMember`, `memberSessions`, `revokeMemberSession`) |
| Roles | No | No: only the owner promotes or demotes |
| Lead bot, roster sections, account and cloud settings | No | No |

Every check is in the host: `people::check_bot` lets an admin name any bot, `api::events` gives an
admin every bot and chat, and `api::marketplace` lets an admin manage every connector. A member keeps
the scoping below. `people` also returns `canManageRoles`, false on an admin's phone, so a client can
show the role without offering to change it.

## What a member's app sees

A device or session assigned to a person gets the same calls as the owner's phone, but scoped in the
host:

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
- **Who am I.** `memberProfile` and `hello.member` say whether the caller is the owner or a person,
  and which role and scopes it holds, so a client shows "Ana, member" instead of guessing.

The owner and this computer still see everything, and an admin's phone sees every bot and chat too:
that is what owning the computer, or being trusted with it, means. Removing a person is how their
access ends; their bots and chats stay (see below).

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

**Settings → People → Remove** asks first, then ends their access and unassigns everything instead of
deleting it:

- Their devices and sessions are revoked, with their push tickets and invitation codes: no phone or
  token of theirs can call the host again. Pair it again if it should come back as the owner's.
- Their bots and chats become the owner's; no bot or conversation is deleted.
- Their connectors become shared, so every bot on the computer can use them.

The confirmation says so before the call. Removing a person never deletes a connector, a bot or a
chat.

## Clients

- **Desktop:** Settings → People: add a person, rename or change their email, give them the admin or
  member role, remove one, and assign a paired device from the row. **Invite a member** shows the
  one-time code and the invitation link (or QR) to hand over. **Member sessions** lists invitations,
  paired phones and signed-in apps with **End session**. **Sign in as a member on this computer**
  takes the invitation code or the company email and opens that member's own view (their connectors,
  and who they are), which the host scopes. Extensions shows a person badge on each connector (or
  **Shared**), and a member's view says **Shared** and **Mine**.
- **iPhone and Android:** Computer management → People, with the same actions: people and roles,
  **Invite a member** (the code and link), **Member sessions** with **End session**, and assigning a
  device. **Sign in as a member** on Computer management scans the invitation with the normal pairing
  screen. An admin's phone sees roles but can't change them; the section is hidden for a member.
- **Terminal UI:** Go to → People on this computer: the people, their role, their email and the
  sessions and devices each one uses. **n** adds someone, **i** invites a member (the sheet keeps the
  one-time code on screen), **↵** renames or changes their email, **g** makes them an admin or a
  member, **x** removes them, **Tab** to the sessions list, **v** ends one session (after asking), and
  **↵** there gives a device to a person or back to this computer's owner. The terminal UI is the
  owner's own view, so it hands out invitations and ends sessions rather than opening one as a
  member: a member signs in on their own phone or with the desktop app.

The desktop app and the phones talk to the same host methods; every check is in the host, never in a
client.

## Limits

- A session token is for this computer's local API: the desktop app and the terminal UI use it over
  loopback or their own tunnel. A phone or a remote app pairs instead, so its traffic stays inside
  the E2E channel. A signed-out phone can only pair on the same network or Tailscale; over the Codync
  relay it needs an account, as before.
- A member at the owner's computer can read the host's token file, so their session there is a
  convenience, not a boundary against the owner. The boundary is a member's own remote device.
- The owner and an admin can read every member's chats and use their connectors. The reverse is
  refused, and a member can't list people, invite anyone, or end a session.
- Member invitations are single-use; the desktop and terminal sign-in exchange a code only at the
  computer. An invitation that leaked is only useful until it is redeemed, revoked, or expires.
- Composio connections and skills are computer-wide: a member sees them and can use them in a bot,
  but only the owner connects or removes a Composio app.
- A member's own new bot gets the shared connectors plus their own. It never gets another person's.
