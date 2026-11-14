# harness-sdk (Strands Agents): what it offers Codync

**Verdict: reference only, never a runtime dependency.** Strands Agents is a Python/TypeScript SDK
(now `CopilotKit/harness-sdk`, forked from `strands-agents/harness-sdk`, Apache-2.0) for building an
agent harness: an agent loop, model providers, tools, MCP, streaming, multi-agent patterns and
observability. Codync already owns every one of those pieces in Rust, inside the host, and adds the
parts Strands deliberately leaves to the application: paired devices, approvals, a vault, routines,
channels and a roster. Embedding a second agent runtime would fork the source of truth and break the
"host owns the logic" rule; nothing in Strands is a drop-in for an ACP-driven host.

What it does, checked against the repo's README and package layout (`strands-py/`, `strands-ts/`,
`strands-mcp/`):

| Piece | Strands | Codync today | Use it? |
| --- | --- | --- | --- |
| Agent loop | Model-driven loop with context management and execution limits | `host/src/agent/bot` drives an ACP agent process; the agent owns its own loop | No: the loop belongs to the agent, not the host |
| Model providers | Bedrock, Anthropic, OpenAI, Gemini, custom | The agent's own sign-in or API key, through ACP | No |
| Tools | Typed tools in Python/TS, MCP client | Built-in MCP servers (`chat`, `team`, `manage`, `computer`, `memory`, `routines`, `connectors`) | No: Codync's tools act on host state and approvals |
| Streaming | Bidirectional streaming, events | SSE/events bus to every client, AG-UI in both directions (K1/K2) | No |
| Multi-agent | Patterns and hand-offs | Team tools, groups, routine flows, delegation | No |
| Hooks | Intercept any loop step to log, validate or redirect | Analytics, audit log, approval rules, warning/block hooks | Borrow the idea where a step has no hook yet |
| Guardrails / steering | Pre-execution checks and self-correction | Approval cards, blocklist, credential caution, validation | Conceptually covered; no port needed |
| Structured output | Typed results | Agent output is free text by design | No |

Concrete pieces worth copying as **design notes**, not code:

1. **A hook at each loop step.** Strands traces every decision by default. Codync logs turns and tool
   calls, but the useful gap is a single declared hook point around "before a tool runs" that approval
   rules, the credential caution and the blocklist all share. Codync already has the three checks;
   unifying them behind one seam would make a fourth check cheap. Track it as a refactor, not a
   dependency.
2. **Execution limits as first-class state.** Strands treats step and time limits as part of the loop.
   Codync has turn timeouts and the paused-turn verdict; a single "budget" object per turn would make
   the limits visible to clients. Also a refactor, already partly built.
3. **Steering responses.** When an agent is about to fail, Strands lets the harness hand it a
   correction. Codync's paused-turn and "the provider gave no reason" paths are where this would fit.
   No new runtime is needed; the host can send a follow-up message to the same session.

What we explicitly do **not** take:

- The Python/TypeScript dependency. It would add a second package manager and a second process model
  to a host that is deliberately one Rust binary.
- Its model-provider abstraction. Codync's value is that the person's agent and sign-in stay theirs;
  re-routing models through a new SDK would change that contract.
- Its MCP client. Codync's MCP servers run in the host and act with host permissions; a generic client
  would bypass the approval and vault rules.

**Two-page conclusion:** Strands is a well-built harness for applications that do not have one. Codync
has one, with stricter boundaries (host-owned state, device approvals, a vault, a single binary). The
honest gain is three design patterns — a per-step hook, an explicit turn budget and steering — which
can be adopted in the existing Rust code without the dependency. K8 is therefore a **design-only
note**; no code change is attached to it.

Sources: `github.com/CopilotKit/harness-sdk` (README, package layout, Apache-2.0), checked
2026-11-14.
