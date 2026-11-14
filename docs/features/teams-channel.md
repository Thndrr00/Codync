# Microsoft Teams channel

Teams is a messaging channel over the Microsoft Bot Framework. The host is the bot: an Azure Bot
resource carries the registration, the Bot Framework connector delivers each Teams activity to a
messaging endpoint on this host, and the host answers through the connector's REST API. The app
password stays in the credential vault; the Azure registration is the person's own work and the host
never claims a connection it cannot authenticate.

## What Azure needs

1. Create an **Azure Bot** resource (Azure portal, or the Microsoft 365 Developer Program for a
   test tenant) and copy its **Microsoft app id**.
2. In **Configuration**, set **Messaging endpoint** to this host's public HTTPS address plus
   `/hooks/teams/<connection id>` (the id the settings screen shows). Teams cannot reach a loopback
   address: put the host behind a reverse proxy or tunnel first.
3. Enable the **Microsoft Teams** channel on the bot resource.
4. Give the app registration a **client secret**, or enable a **managed identity** on the
   resource's identity and use its client id. The host gets a token from
   `https://login.microsoftonline.com/<tenant>/oauth2/v2.0/token` (client credentials,
   `scope=https://api.botframework.com/.default`) or from the Azure instance metadata service for
   a managed identity (`resource=https://api.botframework.com`).
5. Install the bot in Teams (the bot's **Test in Web Chat** or the Teams app manifest), then send
   it a message.

The settings screen shows the exact endpoint path and these steps (`registration` in
`teamsStatus`). Nothing is saved unless Microsoft accepts the app id and secret: `setTeamsChannel`
gets a token first.

## Settings

**Channels** shows the connection's app id, tenant, the Codync bot that answers, the people and
tenants allowed, its memory and picture, the messaging endpoint, its state and the last deliveries.
The fields are:

- **Microsoft app id**: the Azure Bot resource's app id, a GUID.
- **Directory tenant**: the tenant used for the token, or empty for a multi-tenant
  `botframework` registration.
- **App client secret** or **managed identity**.
- **People allowed**: AAD object ids, UPNs/emails, Teams user ids or display names, or `*`.
- **Tenants allowed**: tenant ids, or `*`.
- **What the bot knows about Teams**: given to the bot before every Teams message.

At least one person or tenant list must be present. A tenant list bounds every user; a user list
bounds every tenant. The secret is in the vault slot `teams.channel`. Desktop, iPhone and Android
have the full form; the terminal UI shows the state and the endpoint and can disconnect it (`t`).

## Incoming activities

`POST /hooks/teams/{id}` is outside the local API's rebinding and browser-origin guards, because
the Bot Framework posts to a public host name. It authenticates the call itself:

- `Authorization: Bearer <JWT>` is required. The host fetches
  `https://login.botframework.com/v1/.well-known/openidconfiguration` and its `jwks_uri`, verifies
  the RS256 signature with the published `kid`, then checks:
  - `aud` is this connection's app id;
  - `iss` is `https://api.botframework.com`;
  - `serviceurl` equals the activity's `serviceUrl` (trailing slash ignored);
  - `exp` is still ahead and `nbf` is not in the future (five minutes of clock skew).
- Only `type: "message"` activities are read. Conversation updates are acknowledged and ignored.
- The sender and tenant must be allowed. The conversation reference
  (`serviceUrl`, conversation id, tenant, user id and name) is kept in `teams_refs` for replies.
- The text has the bot's `<at>` mention removed. Attachments contribute their names (not
  downloaded yet). In a group or channel, a mention is required; a personal chat needs none.
- A card submit is `value.codyncAction`, the same `p|<entry>|<option>|<print>` code as Slack,
  Discord and Telegram. The host checks the card's print and that the card was shown in this
  conversation (`teams_cards`), then calls `respondPermission`. The first answer from any client
  wins.

## Replies and cards

Text replies go to `POST {serviceUrl}/v3/conversations/{conversation}/activities`, with
`replyToId` naming the message that started the turn. An approval card is the Adaptive Card built
by `channels::render::teams` (a `TextBlock`, a `FactSet`, a `Table`, an `emphasis` container and one
`Action.Submit` per option, whose `data.codyncAction` is the approval code). When the card settles,
the host replaces the same activity with `PUT .../activities/{activity id}` and a text-only card.

Teams files are not sent yet: a reply that carried files adds a line naming them as not sent, so
the person knows to open them in Codync. The outbox (`teams_outbox`) reports `sending`, `sent`,
`failed` or `unknown`; an interrupted send is never repeated automatically.

## Limits

- Teams needs a public HTTPS endpoint and an Azure Bot registration; Codync cannot create either.
- Managed identity uses the instance metadata service, which is link-local and deliberately outside
  the normal egress policy; only the fixed IMDS URL is called, with redirects disabled.
- Messages from before the host connected are not replayed. The inbox is keyed by activity id.
- Incoming attachments are named, not downloaded. Outgoing files are not supported yet.
- Teams' card actions are the only approval path; there is no numbered-reply fallback (Teams
  always renders the card's buttons).
- Proactive messages outside a conversation need a conversation reference the host has seen; the
  host sends only answers to activities it accepted.

## Files

- `host/src/teams/`: `mod.rs` (vault config, `teamsStatus`/`setTeamsChannel`/`disconnectTeams`, the
  registration text), `client.rs` (tokens, JWKS/JWT verification, activity posts and card updates),
  `activity.rs` (text, mentions, sender and tenant policy, card action), `inbound.rs` (the public
  route and the card answer), `worker.rs` (cards, outbox and replies).
- `host/src/store/teams.rs` and `teams.sql`: inbox, threads, conversation references, outbox and
  card map.
- Tests: activity parsing and allow rules (`activity.rs`), token claims and activity bodies
  (`client.rs`), connection validation and the registration text (`tests.rs`).
