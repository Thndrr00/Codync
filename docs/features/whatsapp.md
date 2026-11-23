# WhatsApp

A computer can link a WhatsApp number as another device; the host runs the link and turns its
messages into chat and routine events. The number's page in the desktop, iPhone and Android apps
lets you choose the bot, allow chats and set the number's own switches.

## Resent group messages

A newly linked number may first receive a group message it can't read yet. WhatsApp sends the
message again once it can (OpenBot 0.70.0): the empty first copy leaves no history row, and the
readable resend within ten minutes is saved in the group history and mentioned to the orchestrator
instead of being dropped as a duplicate. Ordinary repeats are still dropped.

## A link that stops without saying so

WhatsApp can stop handing a number its messages while the link still says "Connected" (OpenBot nightly
triage-12196). The host treats both a message it could not finish (an inbox entry left queued for
30 minutes) and a post WhatsApp never confirmed (a receipt that timed out or came back without a
message id) as a stop: it connects the same number again by itself, without new pairing, so held
messages come in. Three stops inside an hour add **Needs attention** beside the linked number in the
iPhone and Android WhatsApp pages and an Extensions summary, with a **Reconnect** button that does
the same restart on demand. The flag clears on a manual reconnect; the stop count drops out of the
window after an hour. A number on a joined server needs that server on this version too, since the
host owns the decision and the clients only show its state.

## The owner's own chat

The linked number's "Message yourself" chat always reaches the bot, whether the
number is personal or used only for OpenBot, and even when an allowlist of other
chats is set (OpenBot nightly triage-13865). The same pure decision is used by the
host's inbox, live and routine paths, so the chat cannot be filtered out by a
change to the allowlist. It is shown as the first place in the number's settings
on desktop, iPhone and Android, labelled **Message yourself**, and cannot be
removed with **Remove**. The number's settings say that this chat and the people
you allow reach the bot; the other chats still get no reply.

## Read-only chats

An allowed contact or group can be set to **Read only** on the WhatsApp page in the desktop app
(OpenBot 0.77.0). The host still keeps that chat's messages, authors and files for an agent to read
back, but opens no bot turn there: no answer, "Working on it…" line, acknowledgement or error post.
The setting lives on the allowed-list entry (`whatsapp.allowed`), so older plain-jid lists keep
working and are read as writable. It is additive over the API: `whatsappState` also reports
`readOnly`, and `whatsappSetAllowed` takes an optional `readOnly` flag. The jid shape and the
200-chat cap are unchanged.

## Contact cards

A contact card someone shares now reaches the agent as one readable line with the contact's name and
phone numbers, in the live prompt and in the kept history (OpenBot 0.77.0). Before, it arrived as an
empty message. The bridge sends only the name and vCard; the host parses them into text. The numbers
go into the agent's prompt and the history row, never into a log. Cards saved before this release
stay empty.

## Replies to the number's own posts

In an allowed group, a reply to a post an agent or a routine made reaches the configured WhatsApp
bot (the orchestrator) even without a mention (OpenBot 0.77.0). The reply names the post it answers,
so the orchestrator sees which agent wrote it and routes it. The bot is the one already chosen for
the number; no second bot is created.

## The busy note

The "too many requests" note stays out of another person's chat or a group on a personal number and
when the "Working on it…" message is off (OpenBot 0.77.0). Before, it appeared as the owner's own
message there. The failed reaction is unchanged; the owner's "Message yourself" chat and a number
used only for OpenBot still get the note while status posts are on.

## Status reactions

A number's **Status reactions** switch is on by default. While it is on, a request gets a 👀 on
the sender's message when it arrives and a ✅, ❌ or 🫡 when the request ends (OpenBot 0.74.0).
Turning it off stops only those reactions: the answers and the error messages still go out, and
the separate "Working on it…" message keeps its own switch. The setting belongs to the number and
is saved on the computer; changing it in any client takes effect there for every client.
A result routed to a bot that is waiting on a teammate is internal to the computer (OpenBot nightly
triage-13131), so it gets no "Working on it…" line; the 👀 reaction is unchanged, and the owner's own
messages and ordinary contact messages still get the acknowledgement.
