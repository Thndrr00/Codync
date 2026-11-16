# Connection guides

`key_guides.json`, the plain field labels and the "needs a key" wording are ported from
`CopilotKit/OpenBot` (`server/src/plugins/key-guides.ts`, `server/src/plugins/app-names.ts` and
`app/src/lib/plugins/enablement-notes.ts`), MIT-licensed, Copyright (c) 2025 CopilotKit.

What Codync changed: OpenBot calls its broker "OpenBot" in the copy a person reads; here the broker
is Composio, so the provider's own sentence keeps that name. Clause wording is Codync's where a
sentence described OpenBot's approvals and bots.

Every link was opened on 2026-11-08 by the OpenBot authors. The map covers the most-used key-only
apps, not all of them: a link is never guessed, so an app missing from it keeps the vendor's own
help and gets no button.
