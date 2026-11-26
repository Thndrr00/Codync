# OpenBot 0.79.0 in Codync

Source: tag `v0.79.0` (284 bullets) plus `changelog.d` on `origin/main@2b31a27f8`, audited against the
220 commits already on this branch. Rule: copy behavior with a short source comment, reuse Codync's
code/APIs/components, pure tested decisions wired to real call sites. Excluded with reasons: credits,
Chrome extension/X, joined servers, hosted/account service, web client, OS packaging, perf-only.

## State

~130 bullets were already integrated from earlier fragments (Grok tool hang, dictation truncation,
WhatsApp delivery and re-add hints, Telegram reply and busy-post deletion, teammate results, plan mode,
hash links, computer-use permission hint, settings help, workflow nodes/UI/runtime, and more). ~125 are
not portable. The portable candidates below are being integrated in three batches.

## Portable candidates

| # | Behavior | Area |
| --- | --- | --- |
| 1 | Keep an explicit note when an agent ends a turn with no answer | `agent/bot/turn_rules.rs` |
| 2 | Name the concrete channel-lead routing failure and next step | `chat/group.rs`, lead routing |
| 3 | Show provider retries as a warning that clears on recovery | turn notices + desktop notice card |
| 4 | Keep the chosen model/effort on the New agent screen | bot editor |
| 5 | Report why a saved-login routine couldn't fill a password card | `routines/**`, browser cards |
| 6 | "Open update settings" from the idle-update notice | desktop update UI |
| 7 | Browser-page user step prompt (sign-in/CAPTCHA) in the composer | `screen/browser_cards.rs`, composer |
| 8 | Scrub internal text from GPT live-voice requests | `chat/calls/**` |
| 9 | Apply a newly saved API key on the next message after an auth failure | `agent/auth/**` |
| 10 | Show the full workflow step error and a what-to-do hint | `routines/**`, run history |
| 11 | Honor the exact schedule time an agent sets | `routines/**` |
| 12 | Pi context-overflow restart with history | `agent/backends/**` |
| 13 | Mistral chat reopen after GitHub connect | `agent/backends/**` |
| 14 | Read aloud in a separate chat window | `voice.rs` |
| 15 | Hide compact-context when no channel member can compact | `chat/**` channel settings |
| 16 | Bundled Claude Code copy fallback within ~20 s | `agent/backends/**` |
| 17 | Saved login in a masked password field | browser login handling |
| 18 | Right-click to hide/show left-rail items | desktop left rail |
| 19 | Routine report language on every line | `routines/**` |
| 20 | Publish a finished media result even when stopped mid-generation | `media/**` |
| 21 | Gemini busy-notice wording after the turn settles | `turn_rules.rs` |
| 22 | MCP "Sign-in needed" on insufficient tool scope | `mcp/policy.rs` |
| 23 | Connection circles full size at 50% zoom, 2× at 25% | flow canvas |

## Not ported (with reason)

Credits/billing and the credit wallet, redemption codes and crypto wallet; Chrome extension and X;
joined servers and their version gates; hosted server restart/delete and account-service timeouts;
web client; OS packaging (Windows app ID, macOS notch/dock, Linux X11); performance-only internals.
