# Channel rendering

One card model, drawn natively by every messaging channel. The host builds a card once
(`host/src/channels/render.rs`) and each channel's serializer maps its blocks onto what that
channel can show. Approval cards are the first thing rendered this way: the same approval shows
buttons in Slack, Discord components, a Telegram inline keyboard and numbered options in WhatsApp.

## The model

A card is a list of blocks:

| Block | What it is |
| --- | --- |
| `Text` | A line of text. |
| `Fields` | Label/value facts (a tool kind, details). |
| `Table` | Columns and rows; text-only channels show it as `a \| b` lines. |
| `Divider` | A blank line or separator. |
| `Callout` | A tone (`info`, `success`, `warning`, `danger`) and a short text, marked with an emoji. |
| `Buttons` | A label per choice and, optionally, the opaque `action` that pressing it sends back. |

`render::fallback` writes the plain-text form of every block. It is the payload field each channel
shows where rich content is not rendered (Slack's notification text, Discord's `content`), so a
channel that cannot express a block still says everything the card says. A button with no
`action` becomes a numbered option in that text.

## Per-channel payloads

| Channel | Payload | Blocks |
| --- | --- | --- |
| Slack | `chat.postMessage` body: `{text, blocks}` | `section` (`mrkdwn`), `section.fields`, `divider`, `context` for a callout, one `actions` block with a `button` per choice. Button `value` carries the action; text and labels are clipped to Slack's limits. |
| Discord | message body: `{content, embeds, components}` | Text, callout and table in `content`; fields as one embed's `fields`; a `components` action row (`type: 1`) of `type: 2` buttons, five per row. |
| Telegram | `sendMessage` body: `{text, parse_mode: "HTML", reply_markup}` | Text and fields as HTML lines, the table in `<pre>`, callout with its emoji, an `inline_keyboard` with two buttons per row. Callback data is limited to 64 bytes: a longer action leaves that option out, as Telegram would refuse the message. |
| WhatsApp | text | Text, fields as `*label*: value`, the table as lines, callout with its emoji, and buttons as numbered options under `Responde con el número de una opción:`. The existing numbered answer path is unchanged. |
| Matrix | `m.room.message` content: `{msgtype, body, format, formatted_body}` | The block model's plain text as `body` and its HTML form as `formatted_body` (bold labels, `<pre>` table, an ordered list of options). Approvals also get a reaction per option (1️⃣…5️⃣); a numbered reply or a reaction answers them. `host/src/matrix/render.rs`. |
| Teams | Bot Framework message carrying an Adaptive Card | The serializer (`render::teams`) and its wire shape are defined: an attachment with `contentType: application/vnd.microsoft.card.adaptive` and an `AdaptiveCard` 1.5 `body` (`TextBlock`, `FactSet`, `Table`, separator, emphasized `Container` for a callout) plus `Action.Submit` actions whose `data.codyncAction` is the button action. Teams stays out of scope: it needs a bot registered in the Microsoft 365 Developer Program or Azure with a Bot Framework channel, not a token; see [the Matrix channel](matrix-channel.md#teams). |

## Approval cards

`host/src/channels/approval.rs` turns a `permission` entry into a card: `Needs you: <title>`, a
warning callout with the command, fields for the tool kind and details, and one button per option.
Buttons carry `p|<entry id>|<option>|<print>`, where the print digests the title and option ids so
a button from a card that changed cannot answer another. Slack, Discord and Telegram read that code
back; WhatsApp answers by number, and Matrix by number or reaction (the card event is mapped to the
entry in `matrix_cards`).

Every answer still goes through the host's `respondPermission` method, so channel buttons and
Codync's own clients share one answer path and the first answer wins. Telegram's card is sent with
`parse_mode: HTML` (`client::card`); replies are still plain text.

Slack cards need interactivity: the app's request URL must point at `POST /hooks/slack`, or the
host's Socket Mode connection must be on, for Slack to deliver `block_actions`. The host accepts
both, answers Slack within its three-second budget, and settles the card from the entry's event.

## Tests

The serializers have their payloads asserted in `render.rs` (text, buttons and fields for every
channel, plus the fallback), and the approval builder checks the same entry becomes each channel's
buttons. Telegram's card posting and answer path are also exercised against its fake endpoint; the
other channels' payloads are checked here, and their transports keep their own end-to-end checks.
