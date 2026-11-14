# Mini-apps

A mini-app is a small web page a bot builds for you and the host serves from your computer: a split
bill, a tracker, a checklist. It appears in the chat as a card and opens in a viewer of its own, and
it can store its data on the computer. Nothing leaves this machine to run it, and Codync charges
nothing for it.

## What a bot builds

A bot writes the page inside its workspace, under `apps/<slug>/` (HTML, CSS and JavaScript, with
`index.html` by default), then registers it. The host serves the folder under
`http://127.0.0.1:<port>/apps/<bot>/<slug>/<token>/`; the token is the app's owner capability, so a
mini-app is only reachable from the clients that know it.

Registering an app writes a live card in the chat and reloads any viewer already open for it, so a
bot can keep changing an app while you look at it. Each save raises its version.

| Tool | What it does |
| --- | --- |
| `save_app {slug, title, description?, entry?}` | Registers a new app or updates one the bot already has; the entry file must exist under `apps/<slug>/` |
| `list_apps` | This bot's apps with their versions, URLs, collections, and this computer's local app limit and count |
| `delete_app {slug}` | Removes an app, its share links and its collections |
| `app_data_list {slug, collection?, limit?}` | Reads a collection, or lists its collections |
| `app_data_put {slug, collection, id?, value}` | Inserts or updates one item and tells open viewers |
| `app_data_delete {slug, collection, id?}` | Deletes one item, or the whole collection |

An app's page can include the injected `_codync.js` helper and use
`window.codync.{list,get,put,update,remove,tell,subscribe}`. Collections are JSON items stored in
SQLite by `host/src/store/apps.rs`; `tell` sends a message to the bot. The tools and the host API
methods for managing apps and share links are in `host/src/chat/apps.rs`.

## The local app limit

Codync has no plans and hosts no sites, but a computer should not fill its disk with mini-apps by
accident. The bots on a computer share one ceiling, `localApps.limit`, saved in kv by
`host/src/local_apps.rs`:

- The choices are **25, 50, 100, 250 and 1000**, and the default is **100**. The count is every
  registered app on the computer, from every bot, and `botsWithApps` says how many bots made them.
- **Only a new app takes a slot.** Saving an app the bot already registered is an update, and it
  works at the limit; a bot can keep improving what it made.
- At the limit, `save_app` for a new app refuses with the limit, the count, and how to make room:
  "This computer has 100 registered apps and a limit of 100. Delete an app you no longer use, or
  raise the limit in Settings > Local apps."
- Deleting an app makes room at once. Raising the limit adds room without touching an app. Lowering
  it never deletes anything; it only stops new apps until the count is below the new ceiling.
- `list_apps` answers with `count`, `limit`, `choices` and `botsWithApps` beside the apps so an
  agent can see the ceiling before it builds another app.

The host methods are `localApps` (`count`, `limit`, `choices`, `botsWithApps`) and
`setLocalAppsLimit` (`{limit}`). The setting is on the desktop under **Settings > General > Local
apps**, on the iPhone and Android under the computer's settings beside **Local scripts**, and on the
terminal UI's Computer Status page, where `count/limit` shows with the local-scripts line and `<`
and `>` step the limit.

This is deliberately not a hosting product: [F11.16](../guides/openbot-gap-census.md#f1116-local-app-limit)
in the OpenBot gap census records why plans and hosted sites stay out of scope.
