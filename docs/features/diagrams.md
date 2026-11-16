# Diagrams in artifacts

A diagram is an artifact whose content is an Excalidraw scene instead of HTML. A bot makes one when the person
should edit a plan, a flow or a sketch, and the desktop app opens it in a native SVG editor: move shapes, add
rectangles, ellipses, diamonds, arrows and text, change colors, and save. The scene is the same JSON shape Excalidraw
uses, so it can be moved to or from Excalidraw later.

We do not bundle Excalidraw's React app: the licence and bundle size would outweigh the editor. Codync draws the
scene itself and stores the JSON unchanged, so a future richer editor does not need a migration.

## Host

- `publish_diagram {path?, scene?, title?, artifact?}` (`host/src/chat/artifacts.rs`, validation in
  `host/src/chat/diagrams.rs`) creates a diagram artifact, or replaces the scene of one with `artifact`. The answer
  names its id. `list_artifacts` reports `kind: "diagram"`.
- A scene is `{type: "excalidraw", version, source, elements, appState}`. The host rejects anything that is not
  that shape, more than 2,000 elements, non-finite or out-of-range geometry, colors that are not colors, text over
  8,000 characters, and scenes over 2 MiB.
- Storage: diagrams live beside HTML artifacts as `~/.codync/bots/<bot>/artifacts/<id>.scene.json`; the artifact
  metadata gains `kind` (`html` when missing, so older records keep working).
- `saveArtifactScene {botId, artifactId, scene, sha256?}` (Control access) is the owner's edit. `sha256` is the
  version the editor started from; a stale one fails with "the diagram changed elsewhere", never overwriting a newer
  edit. A waiting scheduled replacement of the same diagram is discarded: the person's version wins.
- `readArtifact` returns a diagram's JSON bytes exactly like a page's HTML, checked against its size and SHA-256.

## Apps

- Desktop: a diagram card shows a grid preview and **Diagram**; the viewer renders the scene as SVG with a tool bar
  (Select, Rectangle, Ellipse, Diamond, Arrow, Text), drag to move, an inspector for text and colors, delete, and
  **Save diagram**. A stale save keeps the draft visible and reports the conflict.
- iPhone and Android: a diagram card opens the viewer, which says to edit it on the desktop. They read the scene but
  do not draw it yet.
- Terminal UI: a diagram behaves like any artifact (`o` opens its scene file in the browser as text, `f` saves the
  JSON). There is no terminal editor.

## Limitations

Shapes are rectangles, ellipses, diamonds, arrows, lines, freedraw and text; images, bindings, hand-drawn roughness,
rotation and Excalidraw's panel UI are not drawn. Unknown fields survive a round trip because the host stores the
scene as given.
