# A bot's mailbox

A bot can read, draft and send mail from the person's own Gmail or Outlook without Codync holding a
mail client of its own: the mailbox is one of the connected apps, reached through Composio
(`host/src/market/composio.rs`).

## What the bot does

- `list_apps` shows the mail app when it is connected; `search_tools` finds the exact tool
  (`GMAIL_SEND_EMAIL`, `GMAIL_FETCH_EMAILS`…) and `get_tool` shows its parameters.
- Reading a mailbox is a read-only tool: it runs without asking.
- Sending, replying and deleting change something, so they ask first by default
  ([MCP tool policies](mcp-tool-policies.md)): the person sees the exact tool and its arguments, and
  the approval is claimed once, so a retried send never sends twice.
- Credentials stay with Composio: the host never stores a mailbox password.

## What a bot cannot do

- **IMAP/SMTP of an arbitrary server.** That needs a mail client inside the host and a sign-in to go
  with it, which this build doesn't link. Gmail and Outlook work because Composio already speaks
  them; another provider needs that connector or a new client.
- **Read a mailbox nobody connected.** There is no shared mailbox and no password in a bot's config.

## The flow the pattern asks for

Ask the bot to go through the inbox: it lists the newest messages with the read tool, summarizes
them, and prepares a reply. The reply is a change, so it waits for the person; once they approve,
the host claims that exact request once and sends it. Nothing is sent without an approval, and
nothing is sent twice.

Idea from agentes.chat's agent mailboxes (`packages/engine`, AGPL-3.0-only): the behavior, not the
code.
