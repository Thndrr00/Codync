# OpenDots integration status

Updated 2026-10-24. Source: the OpenDots section of
`docs/nuevo thndr/implementaciónes de otros repos.md` in the main checkout.
The catalog describes product proposals, not an installed dependency.

## Implemented in this worktree

- Items 1 and 3: host-owned bot/group document libraries, subpages, title search,
  pagination, revision checks, autosave and explicit conflict handling.
- Items 4–6: per-document specialist conversations, immutable human review,
  transactional approvals and stable creation/proposal retry identities.
- Item 2: Windows has a visual editor and Markdown/preview modes; Android has
  Markdown editing, previews and block insertions. Both include slash snippets.
  The four catalog commands insert content only. See
  [document protocol and validation](opendots-documents.md).
- Item 28: durable computer-tool activity metadata, limited to 1,000 completed
  records per bot, and CSV export. Arguments, screenshots, outputs and secrets
  are not stored in this audit history. Desktop, Android and TUI have controls
  for activity and export. See [computer use](computer-use.md).
- Additional item E25: every HTTP route checks browser Origin before handling
  requests. Exact localhost/IPv4/IPv6 loopback origins must use the host's own
  port; foreign, malformed, opaque or duplicate origins receive 403. Native
  clients without Origin keep their existing authentication. This does not
  grant CORS access to other origins or forwarded ports.
- Item 31: HTTP/SSE, local stdio and Composio policy enforcement, immutable
  arguments, approval cards, observed read-only defaults and durable single
  dispatch/execution claims. Windows, Android and TUI expose overrides, exact
  request review, bulk revocation and read-only receipt recovery. Missing
  receipts remain unknown and never replay an execution automatically. Native
  integration validation remains pending. See [MCP policies](mcp-tool-policies.md).
- Item 25, partially: host snapshot epochs invalidate captures on takeover,
  handback, final viewer closure, screen disable and bot permission revocation.
  Captures begun before invalidation cannot refresh the current epoch. Actions
  require a new capture after invalidation, with driver-issued element/capture
  IDs checked per bot. The driver transport rechecks control and epoch after
  preparing/connecting, before dispatch. On Windows, revocation terminates the
  owned driver process tree even after dispatch; completed effects cannot be
  undone. Protected entry includes local-owner autofill in a dedicated private
  browser with exact origin/focused-node checks and inherited private pipes.
- Item 26: a host-owned computer panel now exposes state, capability controls,
  workspace browsing/text previews, bounded commands on Windows/TUI, and activity
  with real owner/device actors. Android intentionally offers state/files/activity
  as specified by the catalog. Item 27 is implemented for Codync tool admission,
  lease cancellation and owned shell/driver descendants. Protected browser
  handback closes by default; optional cookie preservation requires a verified
  page reset or falls back to complete closure. See [computer panel](computer-panel.md).
- Item 53: a signed Slack channel, explicit per-channel users/bots, durable
  deduplication, separate conversations and single-attempt outgoing delivery are
  implemented in source. Local setup and the app manifest are available; no live
  workspace was connected. See [Slack channel](slack-channel.md).
- Extras: captured research evidence, a headless runner with exact message result
  attribution, optional voice transcripts, owner-confirmed decision summaries,
  scheduled-message markers, host setup milestones and root onboarding-step/
  abandonment tracking with persistent telemetry. Native/provider validation
  remains pending. See [extras coverage](opendots-extras.md).

Current work excludes iOS, macOS and iPhone by explicit user instruction.
Earlier Apple changes already present in the worktree were left untouched.
Shared host and desktop modules are edited for the Windows/Android/TUI flows;
no Apple-specific implementation changes are part of this continuation.

## Remaining catalog work

- Item 19: persistent isolated computers per bot.
- Item 26: real-app validation of the integrated panel.
- Items 25/27: native host/app validation of protected autofill, driver termination
  and capability revocation. Arbitrary existing browsers/native-app secret fields
  are outside the implemented owned-browser flow.
- Item 31: native/live validation of the expanded proxy, defaults and receipts.
- Item 53: live workspace/HTTPS validation, attachments and OAuth onboarding.
- Extras: live research validation and factual/citation verification, native voice
  and onboarding validation, including real provider delivery;
  other product refinements are not claimed complete.

The current global computer permission remains authoritative. Document approval
does not imply that all agent tools are sandboxed or require approval.

## Validation

The 2026-10-24 focused run passed 44 interpreted host SQL/JavaScript/subprocess
tests and 44 desktop model/collector tests. A real installed Chrome probe passed
with dummy values using the production private-pipe broker and protected-field
JavaScript. This machine required the safe whole-browser-close fallback because
unverified targets survived the page reset. It did not execute the Rust host's
new transport or prove native process cancellation. Cleanup and read-only review
also fixed transport reuse after cancellation and profile-cleanup errors blocking
protected-entry release. Targeted formatting and whitespace checks passed.
See [validation limits and live checks](opendots-validation.md).

Earlier validation below records the state of each preceding continuation.

The latest voice continuation adds opt-in transcript receipts for Windows and
Android, with plain-text unfolding in those clients and TUI. It preserves source
labels and explicitly does not attest playback or decision approval. Six
interpreted collector tests pass; combined focused verification passed 35 host
SQL/subprocess tests and 31 desktop model/collector tests. Native provider/UI
checks and Rust/Kotlin test execution remain pending because no compilation is
permitted. Apple-specific files were not changed in this continuation.

The latest continuation adds production-SQL checks for Slack, research and
setup milestones, interpreted headless subprocess tests, and panel/MCP model
tests. Independent reviews corrected ambiguous Composio success, stale Slack
queue states and attribution of a later unrelated headless answer. Tests do not
replace Rust/Kotlin type checking or live app/provider validation. No compilation
or Apple-specific work was performed.

After the user's no-compilation instruction, interpreted tests, source review
and formatting/whitespace checks were used:

- Desktop Node suite: 184 tests, 165 passed, 19 skipped, zero failures. This
  includes five new tests against the actual document store for normalized
  titles, edits during save, creation replay, mutation locks and CAS conflicts.
- A subsequent focused run passed all six document-store tests, adding coverage
  for a save completing after the panel closes without dispatching more saves.
- Slash-snippet continuation: ten interpreted Node tests passed (four parser
  tests and six store tests). Coverage includes the catalog commands, prefixes,
  literal code/prose, fenced examples and Unicode/Markdown preservation. The
  editor's identity now remains stable when a new page receives its saved ID.
  Cleanup and refactor review also preserved Shift-selection and reused Android
  regex instances. Native tests and real-app focus/IME checks were not run.
- Remote MCP policy continuation: six Node/SQLite tests passed against the
  production schema and SQL transitions. New Rust transport, cancellation,
  immutable-decision and store tests were added but not executed. This does not
  establish Rust type correctness or end-to-end integration. Reviews corrected
  redirect handling, stale connector identities, cancellation during approval,
  configuration races and claimed-permission expiry.
- Document completion continuation: 23 interpreted Node tests passed, including
  the actual Tiptap editor running in JSDOM, plus three production-schema/SQL
  persistence tests. Windows drafts are now durable and scoped, Android models
  share their writer across panels, and review/conversation states are complete
  in source. No compiler, application packaging or native e2e was run.
- Targeted Rust formatting checks and `git diff --check`: passed.
- MCP management continuation: three new owner flows (policy settings/reset,
  paginated exact-request review and confirmed bulk revocation) are implemented
  for Windows, Android and TUI. Nine interpreted model tests and ten SQL tests
  passed. Source review fixed late-dialog retargeting, refresh pagination loss
  and stale revocation cards. Compilation and real-app validation remain pending.
- Parallel cleanup/refactor reviews identified and led to fixes for normalized
  title autosave loops, edits during moves, hidden pending reviews, group library
  access in TUI, main-chat session inheritance and dispatch after invalidation.
- New snapshot, transport and review-list Rust regression tests are source-reviewed but
  were not executed: Rust test execution would require compilation.
- Android checks and real-app end-to-end validation remain pending. No new
  application was compiled, packaged or installed in this continuation.

Earlier, before compilation was prohibited, desktop type checking and a host
binary check passed. A small harness importing production Rust modules passed
five document, two activity and three Origin tests. Full-host test compilation
exhausted Windows memory/paging space; its test executable never ran. Those
earlier results do not validate the subsequent snapshot changes.

The worktree version is already 2.20.0, ahead of the latest v2.19.1 tag; it does
not need another release bump. `origin/dev` does not exist, so the instructed
pull from that branch could not be performed. No commits or pushes were made.
