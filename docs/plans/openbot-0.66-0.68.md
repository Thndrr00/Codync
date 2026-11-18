# OpenBot 0.66.0, 0.67.0 and 0.68.0 on Codync: what was taken

Reviewed 2026-11-15 against Codync `2.47.0`, from OpenBot nightly tags `v0.66.0`, `v0.67.0` and
`v0.68.0`. Same rule as [0.70.0](openbot-0.70.0.md): the license is ignored on purpose, behavior and
text are copied, and the file that holds a piece names where it came from.

These three releases are mostly OpenBot product: the agent dock (Linux floating, channels on the
dock, size slider, drag), the Tasks page, the Vault with two-step codes, joined-server transfer and
the hosted product. Codync has no dock, no Tasks page, no Vault and no joined server, so what maps is
small and is listed here.

## Landed

| Release | Item | Codync shape | Files |
| --- | --- | --- | --- |
| 0.66.0 | Agents now learn skills by themselves: after a correction or a method that worked, the background memory review saves what the agent learned as a new skill, or adds it to one of the agent's own | A dream run reads its batch a second time (`skill_review`): with the bot's `learnSkills` on (off by default, since it spends the bot's tokens in the background and writes instructions nobody asked for), a review decides and a writer writes the skill, saved as that bot's own (`writtenBy`, on for it only, the usual "Saved the skill …" notice). Cautious like `save_skill`: the lesson must cite one of the person's own messages, most reviews save nothing, a new skill is short, an update only touches a skill that bot saved and must keep its text | `host/src/chat/memory/skill_review.rs`, `host/src/chat/memory/dream.rs`, `host/src/store/model.rs` (`learnSkills`) |
| 0.68.0 | A scheduled, script or webhook routine that answers only `[SILENT]` stays quiet, as one that answers `[[no-update]]` does; routines imported from Hermes often use these words | The silence check also reads `[SILENT]`, `[[SILENT]]` and `[[NO-UPDATE]]`, any case, beside the empty answer, `NO_UPDATE` and `(pass)`; anything else is still an answer | `host/src/agent/bot/proactive.rs` (`is_silent`) |
| 0.68.0 | Right-click the island to hide it; the Settings switch shows it again | A right-click anywhere on the island turns it off through the same `island:setEnabled` the Settings switch uses, and Settings › Island keeps the control | `apps/desktop/src/renderer/views/island/IslandWindow.tsx` |
| 0.66.0 | Click an image you attached to see it at full size before you send it | The composer's image tile opens the whole picture in a sheet, on the desktop, Android and the iPhone; the remove button stays in its corner | `ComposerAttachment.tsx` and `composer.css` (desktop), `ComposerAttachment.kt` (Android), `ComposerAttachment.swift` (iPhone) |
| 0.66.0 | Agents can post Discord embeds: a card with a title, a description, a link, a colour, fields, a footer, a time, an author, a thumbnail and an image | A bot's `post_discord_embed {channel, title, description?, url?, color?, fields?, footer?, image?, thumbnail?, author?}` builds the card with Discord's own limits (256-character title, 4096 description, 25 fields, http(s) links only) and posts it with `embeds` when the bot is connected | `host/src/chat/discord_embed.rs`, `discord::send_embed`, `discord/client.rs` (`embed`) |
| 0.67.0 | Copy an image from a chat to the clipboard, from the chat or from its viewer | The picture viewer has **Copy**: the bytes are read the same way the picture is, any format is drawn once and written as PNG, and the sheet says **Copied**. The iPhone puts the picture on `UIPasteboard`; Android writes the file into its share folder and puts the content URI on the clipboard, so another app can paste it | `lib/copy-image.ts`, `views/thread/Attachments.tsx` (desktop), `Thread/Attachments.swift` (iPhone), `AttachmentFiles.kt` (`copy`) and `AttachmentList.kt` (Android), two keys in the 11 catalogs |

## Already here, verified in the code

| Release | Item | Where Codync already does it |
| --- | --- | --- |
| 0.66.0 | A workflow output can post its result into a channel | Routine outputs already take `slack`, `telegram`, `whatsapp`, `discord` and `githubComment`, each validated against its own address shape (`host/src/routines/outputs.rs`). |
| 0.67.0 | Chats with unread replies show first, with a setting to change the order | The roster carries unread counts per row and per section, and the island lists the unread agents (`BotRow.tsx`, `RosterSection.tsx`, `IslandActivity.tsx`); the sections keep the order the person dragged. |
| 0.67.1 | A message to a working teammate tells the sender whether it joined the running task | Not the same tool: Codync's `message_bot` returns as soon as it is queued and says so, and the reply comes with `read_replies`; the answer to "did it join" is Codync's own `steer` on the recipient. |
| 0.68.0 | A routine run can end as failed with a reason | Routine runs already carry a status and a reason, and the chat's routine notice shows it (`host/src/routines/`, `RoutineNotice.tsx`). |

## Not ported, and why

- **The agent dock and its menus** (floating dock on Linux, channels on the dock, dock size, dragging
  agents in it): Codync has no dock; its island hides with a right-click (above) and has no per-item
  menu, and the roster already keeps the person's drag order.
- **The Tasks page** (its composer, agent search, resizable panel, microphone): Codync has no
  separate tasks page; a task is a message in a chat, which already has all of that.
- **Vault two-step codes and the secure sign-in card**: Codync has no Vault.
- **Joined-server work** (transfer with chat history, channels, workflows and per-agent settings;
  editing a custom endpoint on a server; the server's storage list; the web client's Workflows page;
  the mobile thread notification): joined server, as in [0.69.0](openbot-0.69.0.md).
- **The hosted product and releases** (Linux floating dock on XWayland, one-command server install,
  the start-failure recovery window, the staged rollout, the release that must open the previous
  data, the webhook that wakes a hosted server): OpenBot's hosting and packaging.
- **Provider and browser fixes** (Codex reasoning, Claude background subagents and 5-hour limits,
  OpenCode scripts and streams, Grok tool search and blank answers, Kimi, Pi, Gemini cold starts and
  frames, OpenRouter batch models, proxy downloads, antivirus retries, stale browser references,
  snapshot revisions, Computer Use frames and cursor): they live in OpenBot's own page driver and its
  per-provider clients. Codync speaks ACP to the CLI and drives its own browser through a different
  layer, so the fix is the CLI's version or its own change here.
- **Product-only items** (the OpenBot co-author line in commits, the bug-report and idea cards,
  Postiz and the memory connectors, the Ultrafast Speed menu, Claude Fast mode's price
  confirmation): each belongs to Codync's own product decisions, not to a copy of OpenBot's.

## Checks

Host `cargo fmt --check` is clean. The rest of this release's own gate is the same one recorded in
[0.70.0](openbot-0.70.0.md#checks); no client changed in this pass.
