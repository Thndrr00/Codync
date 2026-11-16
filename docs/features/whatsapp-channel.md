# WhatsApp channel

A linked WhatsApp number answers people from the phone through a bot on this computer. The number is a
linked device of that phone; the host starts a small Node bridge (`host/whatsapp-bridge`), owns its state
and turns its messages into chat entries and routine events. WhatsApp has no official client API and may
block a number it takes for automated, so a separate number is safer. The user manual page is
`web/content/docs/whatsapp.mdx`.

## Pairing codes for strangers

A number whose allowed list is empty lets anyone write. With a list, only the owner, the listed people
and the listed groups get through, and an unknown direct sender is asked to wait:

- Their first message gets one line with one short code. Codes use `ABCDEFGHJKLMNPQRSTUVWXYZ23456789`,
  so no `0`/`O` or `1`/`I` is misread.
- A repeat from the same person returns the same code instead of a new one.
- At most three people wait per linked number; a fourth gets nothing until the owner answers one.
- A code waits a day. After that the person needs a new message and gets a new code.
- The requests live in `kv` under `whatsapp.requests`, so they survive a restart. A new linked account
  clears them.

The WhatsApp sheet on desktop, iOS and Android lists the waiting people with **Approve** and **Deny**
(`whatsappState.requests`, answered with `whatsappAnswerRequest`). Approving adds the person to the
allowed chats and sends them one line; denying removes the request and sends nothing. Groups are never
paired: a group that is not allowed is ignored. The decision lives in `host/src/whatsapp/senders.rs`
(codes, pending requests and the one line each person gets) and `host/src/whatsapp/settings.rs`
(approve/deny). The message is stopped at the allow list in `host/src/whatsapp/inbox.rs`, before it is
queued, and `host/src/whatsapp/incoming.rs` repeats the same gate before it reaches a bot.

## Photos, documents and SVG

A reply's file leaves by its extension, in `host/whatsapp-bridge/media.mjs`:

- `.jpg`, `.jpeg`, `.png` and `.webp` go as a photo with its mimetype and, read from the file header,
  its width and height, so WhatsApp does not crop it in the chat.
- `.svg` goes as a document typed `image/svg+xml`, and `.pdf` as a document typed `application/pdf`.
- Every other type goes as a document with the mimetype the caller gave, and its file name.

Inbound, `save_media` in `host/src/whatsapp.rs` names the saved file by the message's mimetype, so an SVG
arrives as `.svg` instead of `.bin`. A size the bridge cannot read still sends the photo, without
dimensions.

## Durable person data

Each linked account keeps one record per sender in `whatsapp_people` (`host/src/store/whatsapp.rs`):
their WhatsApp name and the facts they gave, such as the repository to use or the account to work in.
Every conversation with that person carries the record in the prompt (`host/src/whatsapp/incoming.rs`),
so a new chat, a different bot conversation or a restart does not lose it, and another sender never sees
it.

The bot writes the record with the memory tools `person_remember` and `person_forget_memory`
(`host/src/chat/memory/tools.rs`), only in a WhatsApp turn. A record holds at most 40 facts of at most 500
characters each; saving the same text again corrects it instead of duplicating it. The WhatsApp sheet
shows the saved facts per person, read-only: the bot changes them.

Tests: `host/src/whatsapp/senders.rs` (a code, the same code again, approval, the fourth stranger, a
restart), `host/src/whatsapp/person.rs` (a fact read in another chat and after a restart, another sender
without it, correcting and forgetting), `host/src/whatsapp.rs` (`save_media` maps `image/svg+xml` to
`.svg`), and `host/whatsapp-bridge/media.test.mjs` (photo with dimensions, SVG and PDF documents).
