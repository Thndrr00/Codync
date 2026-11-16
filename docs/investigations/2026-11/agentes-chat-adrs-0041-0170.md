# agentes.chat: the ADRs after 0040

The plan ([plan-agentes-chat](../../plan-agentes-chat.md)) read ADRs 0001–0040 and closed its seven blocks. The
reference repo has 169 ADRs, so 129 were never opened. This is the first pass over them: **the titles were read for
all of them, and five in full** (0117, 0119, 0128, 0131, 0139). A title is not a decision, so nothing below is
adopted on the strength of its title alone.

## Read in full

| ADR | What it decides | What Codync has | Worth it |
|---|---|---|---|
| 0117 injection hardening | Outside text reaches the model cleaned and enveloped; markers can't be forged; a run that read outside is marked; outbound addresses are checked; MCP tool descriptions are pinned | Taken: the envelope, the cleaner, the marker defence, key masking, the prompt line, and the research outbound check ([outside content](../../features/untrusted-content.md)). Missing: pinned tool descriptions | The pinning is the next half; the rest is done |
| 0119 remote MCP server | agentes.chat serves MCP at `/mcp` with its own OAuth 2.1, CIMD and DCR, so Claude, ChatGPT or Cursor act for a person | The host already serves MCP locally (`codync-host mcp network`, `mcp chat`), with the local token; a remote one would live in `cloud/` | Not now: the local path covers the same need for this computer |
| 0128 task manager | One view of every run (queued, running, waiting), what each is doing now, who asked, and stopping it with a reason the agent is told | `botTasks` (a board per bot) and the runtime status; no cross-bot view, no stop-with-reason that reaches the agent | Yes: it is the same hole as the missing global agenda in the OpenBot plan |
| 0131 autonomy brakes | Asking means waiting; noise becomes a digest; repeated items join; parallel work is capped; a costly self-started proposal asks first | Routines have retry and a limit policy; `loop_guard` caps tool loops; nothing about "asked means waiting" | Partly: the "asked means waiting" line is a prompt rule we can carry; the caps are a bigger design |
| 0139 delegated approvals | One person covers another's approvals (always, or while out of office), never the high-risk ones unless allowed, never re-delegated | One person per computer; approvals are the owner's | No: there is nobody to delegate to |

## By title, the ones that look worth opening next

| Area | ADRs |
|---|---|
| Safety and autonomy | 0126 approval requests by agents, 0076 session screening, 0169 agents containers (the OS sandbox block 5 left out) |
| Tasks and long work | 0145 task conversations, 0097 continuation controller, 0089 agents keep going, 0118 long work pauses near the limit, 0121 long work takes itself home, 0045 background work, 0053 routines' unattended work |
| Context and memory | 0050 stable prompt prefix, 0051 compaction with summaries, 0086 agent context window, 0167 memory that proves itself, 0120 agents learn on their own, 0052 person profile |
| Team | 0083 agent groups, 0093 team coordinator, 0137 the team's right hand, 0092 agents propose changes to teammates, 0081 agents propose agents |
| Computers | 0161 computers that sleep, 0168 a computer comes back as it was, 0096 agents own local ports, 0110 browser profiles |
| Channels and events | 0130 connector events as routine triggers, 0106 routines by webhook, 0147 how agents talk to you, 0124 choice cards, 0113 live blocks and polls |
| Use and data | 0103 connector usage, 0141 usage per person, 0115 shared folder, 0157 file sharing and link pages, 0111 drive trash and who can see |

## By title, the ones that don't apply

- Everything that exists because they have organizations, tenants, money, Cloud runners or billing: 0079 billing
  hooks, 0085 workspace roles, 0088 workspace and account deletion, 0094 one setup many agents, 0148 team views and
  sectors, 0154 workspaces across servers, 0163 jobs at scale, 0170 social and ads connectors of our own.
- Everything about their web app as a product: 0105 error pages, 0108 home and start page, 0122 fourteen languages,
  0162 onboarding rail, 0164 new agent card, 0166 person portraits.
- Their own channels' plumbing, where ours already works: 0080 channel categories, 0091 GIFs and stickers, 0112
  messages in a row, 0152 Android push straight to Firebase.

## Order to open them

1. **0128** and **0145** — the task manager and task conversations: they fill the global agenda that the OpenBot plan
   left open, and Codync already has the runs and the routines underneath.
2. **0169** — containers: the only real answer to the module sandbox question (block 5, item 18).
3. **0167** and **0051** — memory that proves itself and compaction with summaries: memory is Codync's most developed
   area, so their lessons land on something real.
4. **0130** and **0106** — connector events and webhook routines: small, and they make routines useful without a
   person typing a schedule.
5. **0137** and **0092** — the team's right hand: only after deciding whether Codync wants a coordinating bot at all.

Nothing here is started. Each needs its own change, its own ADRs read in full, and the four clients kept in step.
