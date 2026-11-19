# OpenBot 0.73.0 on Codync: what was taken

Reviewed 2026-11-18 against Codync `2.47.0`, from the nightly repo at `v0.73.0` (364 commits after
`v0.72.0`). Same rule as [0.70.0](openbot-0.70.0.md): behavior and text are copied, adapted to
Codync's own screens, and the file that holds a piece names where it came from.

0.73.0 is a media and desktop-UX release, and most of it belongs to OpenBot's product: Sunra and
Higgsfield models and credits, hosted storage, discount links and EU VAT, the web client, and a
workflow canvas that several people edit at once on a joined server.

## Landed

| 0.73.0 item | Codync shape | Files |
| --- | --- | --- |
| An image step can use the 4:5 ratio (a social post) and the 5:4 ratio (a print) | Both are in the host's own list (`ASPECTS`), so `/image --aspect 4:5`, a flow's picture step and every client's picker have them without a change there: the clients read `mediaModels` | `host/src/media/mod.rs`, `docs/features/media.md` |
| An image step can be asked for an exact size (64 to 4096 a side), cut to its middle and scaled as a PNG | `/image --size 1024x1280`: the note asks for the nearest aspect ratio and the host crops and scales the file when it comes back; a picture that cannot be read keeps its own | `host/src/media/size.rs`, `command.rs`, `agent/bot/files.rs` |
| A filter beside New in the sidebar: agents or channels, unread, working, chosen folders, and a sort | A filter beside the roster search with the same choices plus needs attention and pinned, a dot when set, the open chat kept, and drag off while filtered. Desktop only: the roster is a desktop surface | `lib/roster-view-filter.ts` and its tests, `views/RosterControls.tsx`, `store/computer-roster.ts` |
| Settings › Appearance › Motion: Background blur and Moving agent faces | The same two switches, per device: blur drops every backdrop-filter through one body attribute, faces rest on their still frame and the group drift stops | `views/settings/SettingsView.tsx`, `lib/prefs.ts`, `lib/use-agent-faces.ts`, `components/Avatar.tsx` |

## Not ported, and why

- **A custom size for an image step** (width and height 64–4096, cropped and scaled to exactly that
  size as a PNG): the next piece of this release to take. It needs a crop-and-scale step on the made
  file (`host/src/media/output.rs`), a size on the command and the step, and the two fields in each
  client's picker. It is a change of its own, not a copy of a list.
- **Sunra and Higgsfield models, their credits and their prices**: OpenBot's Apps marketplace and its
  accounts; Codync's media use the agent's own subscription tools, with no marketplace sign-in.
- **Hosted storage, discount links, EU VAT withdrawal wording, the pricing page**: the hosted product.
- **The web client's storage offer and the web app's settings**: Codync has no web client.
- **The sidebar's side threads under an agent row**: Codync's threads live in the chat's drawer and
  chips with their own states; bringing them into the roster is a desktop change of its own.
- **Several people editing one workflow at once, and the workflow cache**: Codync's flow is edited
  through the host, one writer at a time; there is no second editor to merge.
- **The WhatsApp link wording** (what an unknown person gets on each number kind): a copy change in
  the link sheet, worth taking with its three screens in its own pass.
- **Routing a teammate's answer to the agent that asked without counting it unread**: Codync's team
  replies are read by their sender through `read_replies`, and an answer stops being unread when the
  asker has it; verify the exact case before changing it.

## Checks

Host `rustfmt --check` is clean on the changed file. The branch's own gate is recorded in
[0.70.0](openbot-0.70.0.md#checks); no client changed in this pass.
