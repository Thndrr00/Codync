# OpenBot 0.74.0 in Codync

Source: OpenBot nightly tag `v0.74.0` (2026-11-19), read from `CHANGELOG.md`.
Same rule as the earlier plans: copy behavior and user-facing text with a short source comment,
reuse Codync's own components and bridge to Codync concepts where an OpenBot subsystem does not
exist. "Joined server" and web-client items need a host/server or web client Codync doesn't have.

## Ported — host behavior and status

| OpenBot 0.74.0 | Codync | Where |
| --- | --- | --- |
| A turn that used tools ends, the model checks the request against the work and the answer, and continues unfinished parts at most 3 times; a question or approval still stops it | A bounded self-check after a tool-using turn, capped at 3, never overriding a pending question or approval | `host/src/agent/**`, `host/src/chat/**` |
| "Thinking it through…" beside the avatar with the Natural response style until the first text or step | The activity line of each client, from the host's own status | `host/src/chat/**`, clients |
| "Waiting for background work…" with elapsed time while a Claude agent waits | Same status and elapsed time, driven by the host | `host/src/agent/**`, `host/src/chat/**`, clients |
| "Searching its past chats…" while an agent searches old chats | The existing past-chat search gets that status instead of the web-search wording | `host/src/chat/**`, clients |
| After "New chat" an agent can still search and read the older messages on demand | Search reads the full stored history on demand; each new message still sends less | `host/src/chat/context.rs`, `host/src/chat/history*` |
| A webhook routine can give its signing secret to its agent as an API key, invalidating the old one | The routine settings and card offer the same action through the existing key vault | `host/src/routines/**`, clients |
| An agent can set MCP server headers from a key given in chat, with approval naming the headers and the key masked in logs and results | The existing approval and credential flow, plus the MCP server config | `host/src/mcp/**`, `host/src/chat/manage.rs` |
| An MCP server whose tool schema uses a boolean `true`/`false` field can be added | Normalize boolean fields when testing/adding the server | `host/src/mcp/**` |
| A provider that failed to start several times is tried again on the next message or routine | Reset the failure latch when a new turn or routine starts | `host/src/agent/**` |
| Close a browser tab a blocked earlier action left open | The browser tool's close handles the already-blocked tab | `host/src/chat/browser_tools/**` |
| An agent writes bug reports and feature ideas to GitHub in English | The GitHub report prompt always writes English | `host/src/market/github_skill.rs` or the reporting tool |
| A routine that asks a teammate sends the final answer, including the teammate's reply, to its configured destination | Wait up to a day for the teammate reply before posting | `host/src/routines/**` |
| Clearer message when an earlier page script blocks filling a saved login or card | The browser tool closes other tabs of the site and says which it cannot see | `host/src/chat/browser_tools/**` |
| An agent gets WhatsApp chat lists and group history even when the group has no name | Fall back to "WhatsApp group" instead of `no_answer` | `host/src/whatsapp/**` |
| Clearer message when an MCP sign-in server refuses registration | Point to saving the server in Marketplace and adding a sign-in app | `host/src/market/**`, `host/src/mcp/**` |
| An agent id with one wrong character names the near match | The channel/agent lookup suggests the closest id | `host/src/chat/**` |
| A custom OpenAI-compatible provider with reasoning effort works with newer models | The custom provider's request keeps function tools with reasoning effort | `host/src/agent/**` |
| A Mistral model's Effort menu lists its thinking levels | The model catalog carries the levels and the chosen one applies to the chat | `host/src/agent/**`, clients |

## Ported — desktop UI

| OpenBot 0.74.0 | Codync | Where |
| --- | --- | --- |
| A settings page's Close button is a plain X with no fill or tooltip, filling on hover | The shared modal header's close button | `apps/desktop/src/renderer/components/**` |
| The sidebar button shares the side-panel button's icon, size and shape, and sits at the end of the sidebar while it is open | The shared chrome buttons | `apps/desktop/src/renderer/views/**` |
| Side panel tabs keep one width, show X on selected/hover, scroll when they overflow with an overflow menu, close on middle click, and scroll when dragged to the end | The desktop's tab strip | `apps/desktop/src/renderer/views/**` |
| An agent's settings have an **Apps** row opening Marketplace on Apps | A row linking to the existing marketplace, in all four clients | bots' settings in all clients |
| Threads look like the agent chat: grouped consecutive messages, day separators, working/waiting rows under the last message, steps beside the answer, and a message box naming the agent and "this thread" | The thread view in each client | clients' thread views |

## Ported — workflow canvas

| OpenBot 0.74.0 | Codync | Where |
| --- | --- | --- |
| Group selected nodes (⌘G), name the group, one port per output type, bundled lines with a count | The routine-flow canvas | `apps/desktop/src/renderer/views/flows/**`, `host/src/routines/flow/**` |
| Collapse a group to a stack card showing its pictures, reopen on double-click, ungroup with ⇧⌘G | The same canvas | same |
| Pasting or dropping several files on the canvas or an Asset node groups the new nodes | The same canvas | same |
| A video step's Length menu has Custom, a slider over every length the model makes | The media step's length control | flows UI, host media |

## Not ported — no Codync equivalent

- A server telling members why it could not start, Team API protocol 10, old-release direct-connection refusal, a joined server's version gates: Codync's host and its own protocol have no joined-server/version pairing.
- Invite friends, referral credit, invite codes on sign-in, the sign-in language/privacy line, first sign-in listing existing servers: Codync has no invite-credit or server-list account model.
- The Singapore relay, hosted-server storage and billing, Terms & Conditions text, 10 TB transfer and plan-deletion rules: Codync's relay, account and legal text are its own.
- Umbrel, the web client, Higgsfield/Sunra credit rails, the agent dock chat, the Library side-panel changes, and the hosted low-memory browser tab cap: no web client, no such billing or Library, no dock.
- The CRM form fixes and the server-side small-picture copies of Workflows: no CRM, and Codync's flows don't have a joined server.
