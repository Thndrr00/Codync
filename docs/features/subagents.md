# Subagents

A bot can hand a bounded task to a short-lived child turn of its own agent, wait for the child's
final answer, and use that answer in the same turn. This is the `delegate` tool on the `team` MCP
server: `delegate(task, agent?, depth?)`. `task` is a self-contained request; `agent` optionally
names a different agent backend (it defaults to the calling bot's own agent); `depth` is
an internal field a child reports when it is asked to delegate in turn. The child runs in a fresh
throwaway ACP session on the host, reusing the same turn runner the host uses for background jobs
(so the model is chosen the way it is for those jobs):
it has no chat of its own, no saved memory and no MCP tools, so it cannot send messages to the user,
message the user's other bots, or start a further subagent. Its final reply is returned straight to
the calling bot as the tool result. Unlike `message_bot`, a subagent is the bot's own private helper,
not another bot in the user's roster, so it does not post notices into any user chat.

This is off by default. The bot's owner enables the `subagents` setting in the bot's settings
(`BotConfig.subagents`); until then `delegate` refuses the call. A child spends the bot's own tokens
on its agent and model (or the chosen one), so enabling it can raise the bot's usage.

Two host-side caps keep delegation bounded, mirroring OpenBot nightly's subagent bounds:

- a child may be at most **2 levels** deep (depths 0 and 1 may delegate; depth 2 cannot), and
- a single turn may start at most **4 children**.

The caps are enforced by the host, not by asking the bot nicely: a refused call returns a plain text
result saying which cap was reached. Each attempt reports one `subagent_delegated` analytics event
with only a fixed outcome (`ok`, `depth_limit`, `cap` or `failure`) and the agent id — never the task
text or the child's answer. The child budget resets at the start of every turn
(`chat::team::requests`).
