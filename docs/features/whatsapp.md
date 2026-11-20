# WhatsApp

A computer can link a WhatsApp number as another device; the host runs the link and turns its
messages into chat and routine events. The number's page in the desktop, iPhone and Android apps
lets you choose the bot, allow chats and set the number's own switches.

## Resent group messages

A newly linked number may first receive a group message it can't read yet. WhatsApp sends the
message again once it can (OpenBot 0.70.0): the empty first copy leaves no history row, and the
readable resend within ten minutes is saved in the group history and mentioned to the orchestrator
instead of being dropped as a duplicate. Ordinary repeats are still dropped.

## Status reactions

A number's **Status reactions** switch is on by default. While it is on, a request gets a 👀 on
the sender's message when it arrives and a ✅, ❌ or 🫡 when the request ends (OpenBot 0.74.0).
Turning it off stops only those reactions: the answers and the error messages still go out, and
the separate "Working on it…" message keeps its own switch. The setting belongs to the number and
is saved on the computer; changing it in any client takes effect there for every client.
