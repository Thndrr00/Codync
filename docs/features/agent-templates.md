# Agent templates

An agent template is a bot's *setup* saved on one computer and, if its owner
chooses, shared by a link. It is a starting point for a new bot, not a copy of a
bot: no chat, credentials, accounts, images or computer paths travel with it.
The host owns all of it (`host/src/market/templates.rs`, `host/src/store/templates.rs`,
`host/src/api/templates.rs`); clients only send intents and render what comes back.

## What is saved

A template holds a 22-character share id, a name, a short summary, the bot's
instructions, an avatar colour and shape from Codync's own palette, optional
agent and model defaults, and bounded lists of skill and connector ids. At most
100 templates fit on a computer; names run to 80 characters, summaries to 280,
instructions to 16,000, and each list to 32 ids. Everything is sanitized before
it is stored, so a token, key, private-key block, path, email or URL carrying
credentials in a bot's instructions is redacted (`[redacted]`) rather than
carried into the template. A template is `local` until it is published, then
`published` with a share link and a publication time.

The list is one stamped kv slot (`agentTemplates`), created on first read. Its
idempotent creation means a fresh database and a reopened one behave the same;
each write moves the global rev, so already-synced clients pick changes up
without a new event or a schema migration.

## Sharing

`templatePublish` sets the visibility to `published` and returns
`{templateId, shareUrl, publishedAt}`. The link is
`https://codync.dev/agents/<id>`; the custom-scheme form is
`codync://agents/<id>`. A ref is accepted as a bare 22-character id or one of
those two forms (plus a local `http://localhost` dev origin). Anything else —
another host, a wrong scheme, userinfo, a port on the public origin, a query, a
fragment or an extra path segment — is refused. Only a published template can
be resolved from a share ref; a local one stays addressable by its own id.

Publishing and unpublishing are idempotent: repeating either leaves the same
state, as a retried call from a phone should.

## Installing

`templateInstall` resolves the template, then makes a new bot through
`agent::new_bot::with_defaults` and `Hub::create_bot` — the same path as
`createBot`. The new bot gets a fresh name (`<name> (from template)`, with a
number when that is taken), a fresh workspace and its own defaults. Skill and
connector ids that are not installed on this computer are left out; no command,
working directory, account, credential or avatar image is copied. Installing
reports the existing `bot_created` event with fixed properties only (`kind`,
`via: template`): no new analytics event and no content from the template.

## API methods

| Method | Arguments | Answer |
|---|---|---|
| `templateSave` | `botId`, optional `templateId`, optional `description` | `{template}` |
| `templateList` | — | `{items}` of summaries |
| `templateGet` | `templateId` or `ref` | full detail |
| `templatePreview` | `botId` **or** `templateId` **or** `ref` | sanitized detail |
| `templateRename` | `templateId`, `name` | full detail |
| `templateDelete` | `templateId` | `{deleted}` |
| `templatePublish` | `templateId` | `{templateId, shareUrl, publishedAt}` |
| `templateUnpublish` | `templateId` | `{templateId, visibility}` |
| `templateInstall` | `templateId` or `ref` | `{bot}` |

Every method is validated in the host. The share-id and origin rules mirror
OpenBot nightly's `packages/contracts/src/agent-template-links.ts`.
