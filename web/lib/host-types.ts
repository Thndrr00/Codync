// Wire types for the Codync host API (`host/src/api/`), narrowed to what the web client reads.
// The host serializes camelCase JSON; timestamps are epoch milliseconds. Unknown fields are ignored,
// and an older host's missing fields fall back in `normalizeBot`/`normalizeEvent`.

export type BotStatus = "idle" | "working" | "needsInput" | "error";

export interface PermissionOption {
  optionId: string;
  name: string;
  /** allow_once | allow_always | reject_once | reject_always */
  kind: string;
}

export interface EntryData {
  text?: string;
  final?: boolean;
  status?: string;
  /** A notice the chat hides (scheduled input, provisional output). */
  internal?: boolean;
  title?: string;
  detail?: string;
  author?: string;
  source?: string;
  style?: string;
  options?: PermissionOption[];
  selected?: string;
  /** A tool call's command, shown under an approval card. */
  command?: string;
  cwd?: string;
  toolKind?: string;
  /** user: the message this one replies to. */
  replyTo?: { id: string; author: string; text: string; artifact?: string };
  attachments?: { id: string; name: string; size: number; sha256?: string }[];
  files?: { id: string; name: string; size: number; sha256?: string }[];
  artifact?: { id: string; title: string; updatedAt: number; kind?: "html" | "diagram" };
  thread?: { count: number; unread: number; lastAt: number; authors: string[] };
  routineId?: string;
  runId?: string;
  retry?: boolean;
  cancelled?: boolean;
  memory?: unknown;
  memoryFile?: unknown;
  compaction?: unknown;
  handoff?: { status: string };
  subagents?: { id: string; task: string; state: string; step: string }[];
  transcript?: unknown[];
  [key: string]: unknown;
}

export interface Entry {
  id: string;
  seq: number;
  botId: string;
  rev: number;
  /** user | agent | thought | tool | plan | permission | notice | subagents */
  kind: string;
  turn: number;
  data: EntryData;
  createdAt: number;
  updatedAt: number;
  threadId?: string | null;
}

export interface Bot {
  id: string;
  /** agent or group. */
  kind: string;
  members: string[];
  name: string;
  description: string;
  avatarColor: string;
  avatarShape: string;
  avatarLook: string | null;
  avatarPhoto: string | null;
  backend: string;
  model: string | null;
  cwd: string;
  managedWorkspace: boolean;
  permission: string;
  section: string | null;
  pinned: boolean;
  lead: boolean;
  hidden: boolean;
  archived: boolean;
  status: BotStatus;
  activity: string;
  unread: number;
  lastMessage: string | null;
  lastAt: number;
  createdAt: number;
  rev: number;
  person: string | null;
  memoryRev: number;
}

export interface Hello {
  hostId: string;
  name: string;
  version: string;
  os: string;
  device?: string;
  computerId?: string;
  home?: string;
  urls?: string[];
  minApp?: string;
  canUpdate?: boolean;
  latestVersion?: string | null;
  analytics?: boolean;
}

export interface Artifact {
  id: string;
  botId: string;
  title: string;
  /** Missing on an older host: a page. */
  kind?: "html" | "diagram";
  size: number;
  sha256: string;
  createdAt: number;
  updatedAt: number;
  entryId?: string;
}

export interface ArtifactChunk {
  data: string;
  size: number;
  artifact: Artifact;
}

interface LibraryBase {
  at: number;
}

export type LibraryItem =
  | (LibraryBase & { kind: "artifact"; artifact: Artifact })
  | (LibraryBase & { kind: "file"; entryId: string; file: { id: string; name: string; size: number; sha256?: string } })
  | (LibraryBase & { kind: "attachment"; entryId: string; file: { id: string; name: string; size: number; sha256?: string } });

/** `serverLibrary` adds the bot that shared each item. */
export type ServerLibraryItem =
  | (LibraryBase & { kind: "artifact"; artifact: Artifact } & LibraryBot)
  | (LibraryBase & { kind: "file"; entryId: string; file: { id: string; name: string; size: number; sha256?: string } } & LibraryBot)
  | (LibraryBase & {
      kind: "attachment";
      entryId: string;
      file: { id: string; name: string; size: number; sha256?: string };
    } & LibraryBot);

interface LibraryBot {
  botId: string;
  botName: string;
  image: boolean;
}

export interface RoutineTrigger {
  type: string;
  seconds?: number;
  expression?: string;
  timeZone?: string;
  at?: number;
}

export interface Routine {
  id: string;
  botId: string;
  name: string;
  instruction: string;
  triggers: RoutineTrigger[];
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
  /** Human sentences the host makes of `triggers`. */
  triggerDescriptions?: string[];
  nextRunAt?: number | null;
  lastError?: string | null;
  thread?: boolean;
}

export interface RoutineRun {
  id: string;
  routineId: string;
  botId: string;
  status: string;
  createdAt: number;
  startedAt?: number | null;
  finishedAt?: number | null;
  detail?: string | null;
  threadRoot?: string | null;
  attempt?: number;
  retryable?: boolean;
  failureCode?: string | null;
}

export interface RoutinesAnswer {
  routines: Routine[];
  runs: RoutineRun[];
}

export interface MemoryFact {
  id: string;
  bot: string | null;
  text: string;
  /** user | bot | dream | imported */
  author: string;
  authorBot?: string | null;
  createdAt: number;
}

export interface MemoryLayer {
  entries: MemoryFact[];
  used: number;
  cap: number;
}

export interface MemoryView {
  shared: boolean;
  yours: MemoryLayer;
  bot: MemoryLayer;
  channel?: MemoryLayer | null;
  files?: unknown[];
  dreams?: unknown[];
  projectMemory?: { path: string; text: string } | null;
  memoryRev: number;
}

export interface BotTasks {
  approvals: { id: string; title: string; detail: string; since: number }[];
  runs: RoutineRun[];
  threads: { id: string; unread: number }[];
}

export interface Usage {
  providers: unknown[];
}

export type HostEvent =
  | {
      type: "hello";
      hostId: string;
      computerId?: string;
      rev: number;
      catchUpRev: number;
      catchUp: number;
      usage?: Usage;
      screen?: unknown;
    }
  | { type: "icons"; iconsRevision: number }
  | { type: "bot"; bot: Bot }
  | { type: "botDeleted"; id: string; rev: number }
  | { type: "entry"; entry: Entry }
  | { type: "entryDeleted"; id: string; rev: number }
  | { type: "historyCleared"; botId: string; rev: number; deleted: number }
  | { type: "artifact"; artifact: Artifact; rev?: number }
  | { type: "routineFlow"; rev: number; botIds: string[] }
  | { type: "usage"; usage: Usage; rev?: number }
  | { type: "resync" };
