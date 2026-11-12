# Group teams: one voice and a hierarchy

A group chat can stay a round table (today's behaviour) or work as a team with a lead, roles and
reporting lines. The setting lives in the group's own config (`team`), so every client reads the same
structure and the host enforces it.

## The three modes

- **Round table** (`mode: "roundtable"`, or no `team` at all): the original behaviour, unchanged. Up to
  three rounds, members answer one at a time, `@mentions` pick who speaks, `(pass)` stays out of the room.
- **Smart** (`mode: "smart"`, the default a new team setup starts with): who answers follows the request,
  and every answer lands in the group chat.
  - `/boss`, `/jefe`, `/lead`, `/supervisor`: only the lead answers, with the whole team's context.
  - A role or a name (`"¿cómo vamos con el UI/UX?"`, `"que lo vea el auditor"`, `"Dani, ¿cómo va?"`):
    exactly that member answers, in the group.
  - Nothing in particular (`"¿cómo vamos?"`, `"¿cómo va el equipo?"`): every member answers in the group,
    like a stand-up.
  - An `@mention` is already a direct address and wins over role matching.
- **One voice** (`mode: "team"`): the lead answers once for everybody, may consult members with
  `ask_bot` inside the turn, and may hand the floor to a member by naming it (see `voice`).

No member ever answers in its own chat: every room turn writes to the group lane.

## Who may speak

`voice` decides whether a member gets its own message after the lead:

| Value | Behaviour |
| --- | --- |
| `lead` (default) | Only the lead speaks. |
| `lead_and_hands` | The lead may name a member with `@Name` at the end of its answer; that member then answers for itself. At most two per turn. |
| `all` | Same hand-off, plus any member the lead names. |

## The chart

The desktop editor is a canvas, not a form: cards move freely, the canvas pans and zooms, and a
reporting line is drawn by dragging the dot under a card onto the member it reports to. Several tops
are allowed; the lead is the card with the star. A card carries a role (presets such as Supervisor,
UI/UX, Código, Auditor, QA, Producto, Datos, Marketing or Soporte, or anything typed), what it is
accountable for, and who it reports to. Members can join or leave the chart without leaving the group.

`team.members` holds one entry per member on the chart:

| Field | Meaning |
| --- | --- |
| `botId` | The member. |
| `role` | Its role, such as "Research" or "Reviewer". |
| `responsibility` | What it is accountable for, in one line. |
| `reportsTo` | The member it reports to; none for the lead. |
| `x`, `y` | Where its card sits on the chart. |

The lead is the group's `chiefBotId`, or its first member. The host rejects a structure that names a
bot outside the group, a member reporting to itself, or reporting lines that loop.

## Clients

- **Desktop**: the group editor has a Team structure section — mode, who may speak, and a draggable
  org chart with roles, managers and a lead. A group in team mode shows a one-line bar above the
  transcript with the lead, the member count and the speaking policy; clicking it opens the chart.
- **Android**: the group editor carries the same settings as a compact list (mode, voice, each role
  and manager, make-lead).
- **iOS**: the group editor carries the mode, voice and per-member role/manager; the visual chart is
  desktop-only.

## Where it lives

- Host: `host/src/store/model.rs` (`TeamSettings`, `TeamMember`), `host/src/chat/group.rs`
  (`team_mode`, `run_team`, `team_prompt`, `hand_raisers`) and `host/src/hub/validate.rs`.
- Desktop: `apps/desktop/src/renderer/views/bots/TeamStructureEditor.tsx`,
  `GroupEditorView.tsx`, `views/thread/TeamBar.tsx`.
- Android: `core/.../Team.kt`, `Management.kt` (`GroupDraft.team`), `GroupEditor.kt`.
- iOS: `Models.swift` (`TeamSettings`, `GroupDraft.team`) and `GroupEditorView.swift`.
