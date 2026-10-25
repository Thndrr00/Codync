# OpenDots validation record

Recorded 2026-10-24 on Windows. Compilation, packaging and Apple-specific work
are excluded by the user's instruction. Source implementation and interpreted
tests do not establish Rust/Kotlin/TypeScript type correctness.

## Completed without compilation

- `node --test host/tests/*.test.mjs`: 44 passed, zero failures.
- Focused desktop model/collector suite: 44 passed, zero failures, using Node's
  experimental TypeScript stripping. Covers computer controls, MCP review,
  research settings, voice collection/decisions and onboarding intent tracking.
- `node tools/verify-protected-browser.mjs`: passed against installed Chrome,
  production broker and protected-field JavaScript with dummy values. No real
  credentials or form submission. Verified inherited pipes without a TCP debug
  endpoint, exact origin/focus rejection, password delivery, page destruction
  and fail-closed handback. Unverified targets required whole-browser closure;
  successful cookie-preserving handback was not demonstrated on this machine.
- Targeted Rust formatting and `git diff --check` passed. Independent cleanup
  and read-only reviews corrected cancelled-pipe reuse and profile-cleanup
  recovery. No compiled Rust or Android test executable ran.

## Live validation still blocked

The installed host at loopback port 19222 reports version 2.19.1, environment
main. Read-only `slackStatus` and `researchStatus` return HTTP 404. That binary
does not contain the worktree's new APIs. It cannot validate this source change,
and an updated binary was not compiled or installed.

`adb devices -l` found no connected Android device or emulator. A configured
Slack test workspace and Parallel credential have not been confirmed. No Slack
message was sent and no real research-provider request was made.

When an updated Windows host/app and an Android device are available, validate:

- Revoke browser permission during an outstanding driver action and verify the
  owned process tree exits. Verify completed external effects are not retried.
- Open protected entry, fill a test saved login/code on the approved HTTPS site,
  reject changed origins/focus, close by default and inspect handback fallback.
- Confirm a voice summary, reopen/restart and verify immutable text, receipt/bot
  attribution and no action execution. Exercise calls with transcripts disabled.
- Background/restart onboarding offline, restore connectivity, verify stable UUID
  delivery and opt-out cancellation without content in event properties.
- Verify signed Slack incoming events, duplicate/restart handling, owner sign-in,
  exact conversation routing and uncertain outgoing-result recovery using an
  explicitly authorized test conversation.
- Verify Parallel search and explicit extraction, immutable source packet lookup,
  missing-source errors and citations in an actual agent response.

Auxiliary Chrome test profiles can remain in the system temporary directory
when Windows holds their files. One recursive cleanup attempt was rejected by
automatic policy; the profile was retained rather than bypassing that decision.
