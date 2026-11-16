# Slack channel

The host accepts signed Slack Events API deliveries at `POST /hooks/slack` and
routes allowed people to an assigned bot. This is implemented in source; no
workspace was connected or live Slack message sent during development.

## Local owner setup

Create a Slack app using `packaging/slack-app-manifest.json`. Replace its URL
with the owner's HTTPS endpoint forwarding only `/hooks/slack`. The manifest
enables interactivity there too, for approval buttons. The host does
not create a tunnel or change firewall/bind settings. Remove private-channel
events and `groups:history` if unused; invite/install the bot in chosen channels.

Prepare a JSON file with public identifiers only:

```json
{
  "workspaceId": "T0123456789",
  "botUserId": "U0123456789",
  "bindings": [
    { "channelId": "C0123456789", "botId": "YOUR_BOT_ID", "allowedUsers": ["U9876543210"] }
  ]
}
```

`tools/configure-slack.ps1 -ConfigurationPath FILE -HostTokenFile FILE -Port PORT`
prompts locally for token/signing secret using hidden input. They go through
authenticated loopback to the encrypted host vault; never put them in the JSON,
command arguments or chat. `auth.test` verifies workspace/bot identity. The script
was syntax-checked, not run against Slack. Windows script execution policies apply;
the tool does not change machine policy.

Authenticated APIs: `setSlackChannel`, `disconnectSlack` (local only), and
`slackStatus` (Control). Status returns bindings and the latest 50 outgoing
delivery states, never credentials. Replacing configuration starts a new
generation and cancels older queued prompts.

## Routing and durability

The receiver checks raw-body HMAC-SHA256, unique signature/timestamp headers and
a five-minute timestamp window before parsing/acknowledging. Workspace, channel
and user allowlist must match. Bots/subtypes are ignored. Mentions and configured
DM channels open conversations; ordinary channel messages do not wake a bot.
Unmentioned replies are accepted only inside already mapped threads.

Deliveries persist before acknowledgement. Event IDs and stable message IDs
deduplicate retries and overlapping mention/message events. Queued messages may
be rediscovered after restart; queued-message expiry preserves inbox work not
yet reserved for execution. An execution reserved before a crash is never
automatically repeated and can remain interrupted/unknown.

Slack threads never fork the main chat or copy its preceding messages. Assigned
bot tools, workspace and memory are still shared: use a dedicated bot when those
must not be available to allowed Slack users. This is not a per-user sandbox.
Clients render existing host conversation entries; no new channel UI was added.

Only final agent text from mapped threads is posted with original `thread_ts`;
main-chat/partial/other-bot entries are excluded. An outbox claim precedes each
send. Unknown outcomes never resend automatically. Text over 35,000 characters
is marked failed. One workspace, text and explicit DM channel IDs are supported;
attachments and Slack OAuth onboarding remain pending.

A thread can also be linked to a Codync thread (`linkConversation`, see [voice calls](voice-call.md#one-conversation-across-text-a-call-and-a-channel)):
messages and replies then use the linked root, so a call started from that conversation and the Slack thread share
one transcript. The channel's settings list a Slack channel's links, remove one, and link a thread by its timestamp
(the one Slack shows in the thread's permalink) to a Codync thread picked from the chat; the same control exists on the
iPhone and Android channel sheets, and the terminal UI uses the host methods only.

Approvals asked in a mapped thread are posted as a Block Kit card with one button
per option and settled once anyone answers, here or in another Codync client.
Blocks, buttons and their fallback text are described in
[channel rendering](channel-rendering.md). Slack needs the app's request URL or
Socket Mode to deliver `block_actions`; both are accepted.

Six interpreted production-SQL tests cover deduplication, execution claims,
final-thread routing, single attempts, restart and explicit owner sign-in resume.
Only messages waiting for sign-in can be admitted again after that owner action.
A Rust signature test was
added but not run. Live signatures, HTTPS forwarding and Slack delivery need an
authorized workspace and a future validated host build.

References: [signature verification](https://docs.slack.dev/authentication/verifying-requests-from-slack/),
[mentions and direct messages](https://docs.slack.dev/reference/events/app_mention/).
