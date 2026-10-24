# OpenDots extras: source coverage

This continuation targets Windows, Android and shared host logic. No Apple
source is changed and no compiler is run.

## Research with sources (E19)

Chat MCP offers `research_sources {objective,urls?}`. The host uses Parallel's
current `/v1/search` and `/v1/extract` APIs with a vaulted key; the existing agent
synthesizes the brief. Explicit URLs use extraction and never silently fall back
to broad search. Missing requested pages are listed separately; unrelated URLs
cannot replace them.
It returns immutable packets of exact URLs, titles, excerpts, publication dates
when provided, capture time, packet ID and limitations. Instructions require
citations and distinguish inference from evidence; source text is untrusted.
Missing/invalid evidence fails explicitly instead of inventing results.

Limits: 2,000-character objective, 45-second provider deadline, 256 KiB response,
eight HTTPS sources, 4,000-character excerpts and latest 200 packets per bot.
The host does not fetch arbitrary source URLs itself; its credential-bearing
provider request follows no redirects. Evidence is plaintext under transcript
storage assumptions. Saving after bot deletion is refused.

APIs: local-only `setResearchKey {key}` (empty clears), `researchStatus`,
`researchSources {botId,objective,urls?}`, `researchPacket {botId,id}`. Research uses
the owner's provider account and normal API charges. No live call was made.
Windows Settings includes local key add/replace/removal with hidden input.
Android/TUI show host status and use the same chat tool; credential mutation
is local-only. Five interpreted model tests cover status, saves, stale replies,
duplicate clicks and failures. Automatic factual/citation verification remains
pending. One production-SQL test covers persistence scope,
immutability, retention and deletion; Rust parser tests were not run.
References: [Parallel Search](https://docs.parallel.ai/api-reference/search/search),
[Parallel Extract](https://docs.parallel.ai/api-reference/extract/extract).

## Execution without UI (E18)

The existing host already runs without Electron or an Android client. Use:

```text
node tools/run-headless.mjs --token-file PATH --port PORT --bot ID --prompt-file PATH
```

Optional: `--thread ROOT_ID`, `--timeout SECONDS` (600 by default). Credentials
come from a file. The runner sends once and polls read-only; send errors report
the nonce and an unknown outcome. It never automatically resends, cancels a task,
follows redirects, installs or compiles a host.

The host stamps `turnResult` on each initiating user entry, including every
message in a batch. The runner requires this exact message association, never
an unrelated later final response. Exact `messageResult {botId,id}` lookup avoids
history pagination limits. It outputs JSON with message ID, outcome and text.
Waiting for owner sign-in is nonterminal and may complete after authorized resume.
Executing source IDs and the inflight marker commit atomically. On startup,
original requests become interrupted/unknown before automatic session resume,
including tasks too old to resume. The runner never repeats the original prompt.
Hosts lacking the API fail explicitly. Six interpreted subprocess tests cover
success, exact lookup, uncertain send, unrelated results, sign-in and restart;
one SQL test covers atomic marker persistence. This is not a cloud backend or
isolated per-bot computer.

## Voice receipts and confirmed decisions (E20)

Windows/Android pass stable UUID `callId` values. The host stores one immutable
`voice-receipt-<bot>-<call>` notice with `source: voice_receipt`. Identical replay
does not duplicate it; different receipt content is refused. Duration is 0–604800 seconds.
Existing callers without `callId` still work. Native voice validation is pending.

Windows and Android offer **Save call transcripts**, off by default and captured
once at call start. OpenAI Realtime requests input transcription only when enabled;
Gemini requests input/output text; speech modes record recognized input and text
queued for synthesis. Android's device speech is included. No audio is stored by
this feature. Realtime transcription may add provider charges. Changing this
setting affects the next call, including any fallback engines in that call.

The host validates at most 96 segments, 4,000 characters per segment, 32,000 total
and 10,000 reported interruptions. Captures carry truncation metadata. Provider
events can arrive out of order, interrupted generated text may never play, and
text queued for speech may fail synthesis. Receipts explicitly identify these
limits; they are client reports, not proof of audio spoken/heard or approvals.
Stable call IDs are required for transcripts; their first saved payload is immutable.
Windows/Android unfold plain text in chat; TUI uses its existing `o` unfold action.
Apple-specific clients are excluded by the user's instruction and retain the
existing duration receipt. TUI has no audio engine or capture preference.

Six interpreted tests exercise the actual Windows collector: provider event
deduplication, Gemini fragments/whitespace, text/count bounds, detached snapshots,
Unicode boundaries, malformed events and NUL removal. Rust/Kotlin regression tests were added
but not executed because they require compilation. Provider audio and app UI
validation remain pending.

Windows, Android and TUI let the owner write and confirm a decision summary on
any voice receipt, including calls without captured transcripts. The host's
`voiceDecisions` lookup and `confirmVoiceDecisions` mutation bind the summary to
the same bot and receipt. The first confirmed text is immutable and records the
confirming owner/device. A summary never executes an action or treats provider
transcription as approval. Seven interpreted Windows model tests cover loading,
confirmation, stale responses, closing/reopening and unknown outcomes; native
Android/TUI and provider validation remain pending.
References: [OpenAI Realtime transcription](https://developers.openai.com/api/docs/guides/realtime-transcription),
[Gemini Live capabilities](https://ai.google.dev/gemini-api/docs/live-api/capabilities).

## Scheduled markers (E21)

Routine roots/results retain deterministic `routine-<run>` and
`routine-result-<run>` IDs and now carry `source: scheduled_task` plus versioned
run/phase metadata. Their notice/agent roles remain unchanged. History replay
does not execute tasks; the existing durable routine outbox publishes separately
from execution. Markers confer no authority.

## Setup/onboarding telemetry (E27)

Existing onboarding-completed events remain. Host milestones record first bot
readiness and first successful nonempty user answer once per installation.
Assistant/channel-start failures use fixed categories, never error text. They
share existing analytics opt-out, identity and delivery, with no new destination.
One interpreted SQL test covers atomic deduplication and unrelated settings.

Windows tracks root steps welcome, computer access, pairing and ready. Android
tracks welcome, pairing and ready. Backgrounding or recovering an unfinished
step records abandonment. Internal account-provider screens are not separately
instrumented. The host accepts only closed step values and UUID intent IDs;
no screen content, account names or free-form error text are recorded.

Windows persists up to 64 telemetry intents before dispatch. Host delivery uses
a SQLite outbox bounded to 500 events and deduplication receipts bounded to 2,000.
Android persists up to 500 events in preferences. Stable UUIDs survive retries;
network failures, HTTP 408/429 and server failures are retried. Opt-out clears
queues, prevents new admission and cancels active delivery. Four interpreted
tracker tests and four production-SQL tests cover persistence, deduplication,
concurrent acknowledgement, consent and bounded retention. Native delivery and
real onboarding UI checks remain pending. No onboarding UI was added.
