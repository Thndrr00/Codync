# Proactive notices and the interruption policy

A bot that checks in on its own must not become noise. The host keeps one notice per group of findings, never raises
the same topic twice while it waits for an answer, and lets a topic return after the person replies or its cooldown
passes (`host/src/notices.rs`).

## Policy

- **One notice, several findings.** Findings raised within `noticeBatchMinutes` (default 10, settings per bot) share
  one `notice` entry. Its `data.notices` lists each `{topic, text, style}`; the text is "N findings:" followed by one
  bullet per finding, and the strongest style (error over info) wins. At most six findings share a notice.
- **No repeats.** A topic (a trimmed, lowercased string such as `deploy`, `routine:<id>:paused` or
  `memory:MEMORY.md`) is recorded with its notice. While that notice waits, the same topic is suppressed.
- **A reply or a deadline.** A visible user message after the notice releases all its topics, and so does
  `noticeCooldownHours` (default 6) without a reply. Released topics can be raised again, in a new notice.
- **Proactive check-ins.** The scheduler skips a due check-in while the last one produced a visible message that the
  person has not answered and the cooldown has not passed. A check-in that ended in `NO_UPDATE` does not hold the next
  one back. When it runs, its prompt asks for every finding in one message.

The settings live with the proactive settings on every client that has them: **Repeat a topic after** and
**Combine findings within** on desktop and Android. The host's defaults apply when an older client omits them.

## Wiring

- `host/src/proactive.rs`: the check-in gate (`notices::check_in_waiting`).
- `host/src/routines/watch.rs`: a paused watch routine raises a `routine:<id>:paused` finding.
- `host/src/chat/memory/beliefs.rs`: "I learned …" notices and contradiction questions go through `notices::tell`, so
  a repeated memory note does not become another notice. The memory notice keeps its entry-level `memoryUndo`.
- `host/src/chat/memory`'s existing 24-hour question limit still applies before the policy.

State is durable in kv under `notices.<bot>`, so a host restart does not forget what was raised.

## Tests

`host/src/notices.rs` covers three findings in one notice, a repeated topic suppressed within the window, another
finding joining the open notice, a reply releasing a topic, the cooldown releasing it without a reply, a memory
notice keeping its extra fields, and a check-in waiting for a reply.
