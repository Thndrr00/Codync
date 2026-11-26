package com.codync.android.core

import kotlinx.serialization.Serializable

/** A read-only summary of one bot, computed by the host. */
@Serializable
data class BotInsights(
    val botId: String = "",
    val description: String = "",
    val model: String = "",
    val provider: String? = null,
    val recentWork: List<String> = emptyList(),
    val activity: InsightActivity = InsightActivity(),
    val health: InsightHealth = InsightHealth(),
    val profile: InsightProfile = InsightProfile(),
    val week: InsightWeek = InsightWeek(),
    val routineSummary: InsightRoutineSummary = InsightRoutineSummary(),
    val library: InsightLibrary = InsightLibrary(),
    val trust: InsightTrust = InsightTrust(),
    val resilience: InsightResilience = InsightResilience(),
    val identity: InsightIdentity = InsightIdentity(),
    val connectors: InsightConnectors = InsightConnectors(),
    val tuning: InsightTuning = InsightTuning(),
    val memory: InsightMemory = InsightMemory(),
    val workspace: InsightWorkspace = InsightWorkspace(),
    val session: InsightSession = InsightSession(),
    val advice: InsightAdvice = InsightAdvice(),
    val digest: InsightDigest = InsightDigest(),
    val timezone: InsightTimezone = InsightTimezone(),
    val approvals: InsightApprovals = InsightApprovals(),
    val failures: InsightFailures = InsightFailures(),
    val conversation: InsightConversation = InsightConversation(),
    val agent: InsightAgentProfile = InsightAgentProfile(),
    val fresh: InsightFresh = InsightFresh(),
    val prompts: InsightPrompts = InsightPrompts(),
    val directory: InsightDirectory = InsightDirectory(),
    val blocks: InsightBlocks = InsightBlocks(),
    val access: InsightAccess = InsightAccess(),
    val dictation: InsightDictation = InsightDictation(),
    val liveness: InsightLiveness = InsightLiveness(),
    val context: InsightContext = InsightContext(),
    val opendots: InsightOpenDots = InsightOpenDots(),
    val openbot: InsightOpenBot = InsightOpenBot(),
    val agenteschat: InsightAgentesChat = InsightAgentesChat(),
    val textfmt: InsightTextFmt = InsightTextFmt(),
    val localfmt: InsightLocalFmt = InsightLocalFmt(),
    val markdown: InsightMarkdown = InsightMarkdown(),
    val notify: InsightNotify = InsightNotify(),
    val mailtext: InsightMailText = InsightMailText(),
    val connstate: InsightConnState = InsightConnState(),
    val eval: InsightEval = InsightEval(),
    val search: InsightSearch = InsightSearch(),
    val appearance: InsightAppearance = InsightAppearance(),
    val quota: InsightQuota = InsightQuota(),
    val i18n: InsightI18n = InsightI18n(),
    val stream: InsightStream = InsightStream(),
    val blueprint: InsightBlueprint = InsightBlueprint(),
    val review: InsightReview = InsightReview(),
    val history: InsightHistory = InsightHistory(),
    val apierror: InsightApiError = InsightApiError(),
    val memoryctx: InsightMemoryCtx = InsightMemoryCtx(),
    val searchextra: InsightSearchExtra = InsightSearchExtra(),
    val evalextra: InsightEvalExtra = InsightEvalExtra(),
    val connextra: InsightConnExtra = InsightConnExtra(),
    val routines: List<InsightRoutine> = emptyList(),
    val usage: InsightUsage = InsightUsage(),
    val skills: InsightSkills = InsightSkills(),
)

/** A host-computed eval metrics summary from the child-agent port. */
@Serializable
data class InsightEvalExtra(
    val headline: String = "", val tone: String = "", val lines: Int = 0,
    val code: Int = 0, val median: Int = 0, val p90: Int = 0,
    val over1k: Int = 0, val over2k: Int = 0, val skip: Boolean = false,
    val module: String = "", val verdicts: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed connection summary from the child-agent port. */
@Serializable
data class InsightConnExtra(
    val headline: String = "", val tone: String = "",
    val channels: List<String> = emptyList(), val providers: List<String> = emptyList(),
    val selected: Int = 0, val rows: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed ranked-search summary from the child-agent port. */
@Serializable
data class InsightSearchExtra(
    val headline: String = "", val tone: String = "", val query: String = "",
    val terms: List<String> = emptyList(), val total: Int = 0, val returned: Int = 0,
    val hasMore: Boolean = false, val empty: Boolean = false, val score: String = "",
    val totalLabel: String = "", val ops: Int = 0, val sort: String = "",
    val matched: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed memory-context sanitisation summary. */
@Serializable
data class InsightMemoryCtx(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val fenced: Int = 0, val notes: Int = 0, val hidden: Int = 0,
    val preview: String = "", val block: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed provider API error summary. */
@Serializable
data class InsightApiError(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val `class`: String = "", val classLabel: String = "",
    val status: String = "", val retryable: Boolean = false,
    val streamParse: Boolean = false, val network: Boolean = false,
    val samples: Int = 0, val preview: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed history commentary summary. */
@Serializable
data class InsightHistory(
    val headline: String = "", val tone: String = "", val messages: Int = 0,
    val commentary: Int = 0, val finals: Int = 0, val withheld: Int = 0,
    val preview: String = "", val visible: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed idle review queue summary. */
@Serializable
data class InsightReview(
    val headline: String = "", val tone: String = "", val mode: String = "",
    val modeIcon: String = "", val maxAge: Double = 0.0, val settled: Boolean = false,
    val live: Int = 0, val queued: Int = 0, val dispatchable: List<String> = emptyList(),
    val pollMs: Int = 0, val idle: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed automation blueprint summary. */
@Serializable
data class InsightBlueprint(
    val headline: String = "", val tone: String = "", val template: String = "",
    val cron: String = "", val missing: List<String> = emptyList(), val slots: Int = 0,
    val schedule: String = "", val slash: String = "", val deeplink: String = "",
    val time: String? = null, val valid: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed stream delivery summary. */
@Serializable
data class InsightStream(
    val headline: String = "", val tone: String = "", val held: Int = 0,
    val consumers: Int = 0, val chunks: Int = 0, val remaining: String = "",
    val writer: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed locale and plural summary. */
@Serializable
data class InsightI18n(
    val headline: String = "", val tone: String = "", val code: String = "",
    val native: String = "", val english: String = "", val intl: String = "",
    val week: Int = 0, val weekLabel: String = "", val script: String = "",
    val direction: String = "", val rtl: Boolean = false, val plural: String = "",
    val pluralFrom: String = "", val categories: List<String> = emptyList(),
    val locales: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed account quota summary. */
@Serializable
data class InsightQuota(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val usedPct: Int = 0, val remaining: Int = 0, val reset: String = "",
    val resetPhrase: String = "", val depleted: Boolean = false, val low: Boolean = false,
    val scope: String = "", val scopeIcon: String = "", val status: String = "",
    val windows: Int = 0, val bpFull: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed appearance and colour token summary. */
@Serializable
data class InsightAppearance(
    val headline: String = "", val tone: String = "", val preference: String = "",
    val appearance: String = "", val icon: String = "", val background: String = "",
    val text: String = "", val foreground: String = "", val accent: String = "",
    val radius: String = "", val bodySize: Int = 0, val bodyLine: Int = 0,
    val fontScale: Double = 0.0, val maxScale: Double = 0.0, val botColor: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed connection state summary. */
@Serializable
data class InsightConnState(
    val headline: String = "", val tone: String = "", val selected: String = "",
    val state: String = "", val connected: Int = 0, val channels: Int = 0,
    val stale: Int = 0, val backoff: Int = 0, val icon: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed evaluation summary. */
@Serializable
data class InsightEval(
    val headline: String = "", val tone: String = "", val score: Int = 0,
    val verdict: String = "", val threshold: Int = 0, val samples: Int = 0,
    val grade: String = "", val icon: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed search summary. */
@Serializable
data class InsightSearch(
    val headline: String = "", val tone: String = "", val query: String = "",
    val hits: Int = 0, val filtered: Int = 0, val score: Int = 0,
    val empty: Boolean = false, val icon: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed mailbox and database text summary. */
@Serializable
data class InsightMailText(
    val headline: String = "", val tone: String = "", val address: String = "",
    val mailbox: String = "", val domain: String = "", val valid: Boolean = false,
    val keys: Int = 0, val addresses: Int = 0, val rows: Int = 0, val columns: Int = 0,
    val cut: Boolean = false, val maxBody: Int = 0, val maxRows: Int = 0, val maxValue: Int = 0,
    val preview: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed notification and delivery summary. */
@Serializable
data class InsightNotify(
    val headline: String = "", val tone: String = "", val state: String = "",
    val stateIcon: String = "", val marker: String = "",
    val diagnostic: Int = 0, val freeform: Int = 0, val delivered: Int = 0,
    val failed: Int = 0, val muted: Boolean = false,
    val maxOutput: Int = 0, val maxAttempts: Int = 0,
    val floodDefault: Double = 0.0, val floodCap: Double = 0.0, val retryDelay: Double = 0.0,
    val obligation: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed Markdown prose summary. */
@Serializable
data class InsightMarkdown(
    val headline: String = "", val tone: String = "", val preview: String = "",
    val blocks: Int = 0, val links: Int = 0, val mentions: Int = 0,
    val wide: Boolean = false, val heading: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed locale clock and number summary. */
@Serializable
data class InsightLocalFmt(
    val headline: String = "", val tone: String = "", val locale: String = "",
    val clock: String = "", val day: String = "", val weekday: String = "",
    val shortDate: String = "", val month: String = "", val relative: String = "",
    val timeAgo: String = "", val count: String = "", val tokens: String = "",
    val spend: String = "", val budget: String = "", val ordinal: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed text hygiene summary. */
@Serializable
data class InsightTextFmt(
    val headline: String = "", val tone: String = "", val slug: String = "",
    val handle: String = "", val excerpt: String = "",
    val validEmails: Int = 0, val invalidEmails: Int = 0, val hidden: Int = 0,
    val passwordLike: Int = 0, val typoFix: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** Agentes-chat-style context budget summary. */
@Serializable
data class InsightAgentesChat(
    val headline: String = "", val window: String = "", val windowIcon: String = "",
    val windowTone: String = "", val used: String = "", val percent: Int = 0,
    val auto: String = "", val autoTone: String = "", val autoEfforts: List<String> = emptyList(),
    val timeout: Int = 0, val placeKept: Int = 0, val groupWindow: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** OpenBot-style run model summary. */
@Serializable
data class InsightOpenBot(
    val headline: String = "", val tone: String = "", val engine: String = "",
    val model: String = "", val effort: String = "", val effortIcon: String = "",
    val effortTone: String = "", val route: String = "", val routeIcon: String = "",
    val plan: Boolean = false, val thread: String = "", val responsesApi: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** OpenDots-style Learning and internal message summary. */
@Serializable
data class InsightOpenDots(
    val headline: String = "", val container: String = "", val valid: Boolean = false,
    val delivery: Boolean = false, val deliveryError: String = "",
    val learningIcon: String = "", val learningTone: String = "", val internal: Int = 0,
    val scheduledLabel: String = "", val voiceLabel: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed context pruning summary. */
@Serializable
data class InsightContext(
    val headline: String = "", val page: String = "", val screenshots: String = "",
    val pageKeep: Int = 0, val screenshotKeep: Int = 0, val staleMin: Int = 0,
    val pageNote: String = "", val screenshotNote: String = "",
    val pruned: String = "", val tone: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed liveness and delivery summary. */
@Serializable
data class InsightLiveness(
    val headline: String = "", val tone: String = "", val state: String = "",
    val icon: String = "", val active: Boolean = false, val age: String = "",
    val generation: Int = 0, val heartbeat: Int = 0, val due: Boolean = false,
    val outcome: String = "", val outcomeIcon: String = "", val outcomeTone: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** Voice dictation settings from host configuration. */
@Serializable
data class InsightDictation(
    val headline: String = "", val tone: String = "",
    val engine: String = "", val engineIcon: String = "", val engineTone: String = "",
    val model: String = "", val language: String = "", val supported: Boolean = false,
    val needsCommit: Boolean = false, val commitMs: Int = 0, val count: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed public link access summary. */
@Serializable
data class InsightAccess(
    val mode: String = "", val modeIcon: String = "", val modeTone: String = "",
    val canWrite: Boolean = false, val form: Boolean = false, val count: String = "",
    val collections: List<InsightAccessCollection> = emptyList(),
    val invalid: List<String> = emptyList(),
    val routing: String = "", val routingIcon: String = "", val routingTone: String = "",
    val liveName: String = "", val summary: List<InsightSummaryRow> = emptyList(),
)

@Serializable data class InsightAccessCollection(val name: String = "", val valid: Boolean = false)

/** A host-computed transcript view of the last message. */
@Serializable
data class InsightBlocks(
    val headline: String = "", val count: String = "", val said: Int = 0,
    val work: Int = 0, val workOnly: Boolean = false, val quiet: Boolean = false,
    val origin: String = "", val originIcon: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val blocks: List<InsightMessageBlock> = emptyList(),
)

@Serializable data class InsightMessageBlock(
    val kind: String = "", val text: String = "", val icon: String = "",
    val tone: String = "", val says: Boolean = false,
)

/** A host-computed directory summary for one bot. */
@Serializable
data class InsightDirectory(
    val headline: String = "", val tone: String = "", val total: String = "",
    val ready: Int = 0, val unread: Int = 0, val busy: Int = 0,
    val percent: Int = 0, val bar: String = "", val directory: String = "",
    val email: String = "", val routable: Boolean = false,
    val routeIcon: String = "", val routeTone: String = "", val reason: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val integrations: List<InsightDirectoryIntegration> = emptyList(),
)

@Serializable data class InsightDirectoryIntegration(
    val id: String = "", val name: String = "", val endpoint: Boolean = false, val header: String = "",
)

/** A host-computed prompt style summary for one bot. */
@Serializable
data class InsightPrompts(
    val model: String = "", val context: String = "", val price: String = "",
    val valid: Boolean = false, val checkIn: String = "", val zone: String = "",
    val clock: String = "", val watch: String = "", val length: String = "",
    val instruction: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<String> = emptyList(),
)

/** Helpers ported from the latest bdots source, shaped for display. */
@Serializable
data class InsightFresh(
    val agent: String = "",
    val deployment: String = "",
    val language: String = "",
    val receipt: String = "",
    val profile: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val titles: InsightFreshTitles = InsightFreshTitles(),
    val resources: List<InsightFreshResource> = emptyList(),
)

@Serializable data class InsightFreshTitles(val exec: String = "", val write: String = "")
@Serializable data class InsightFreshResource(
    val path: String = "", val kind: String = "",
    val script: Boolean = false, val reference: Boolean = false, val asset: Boolean = false,
)

/** A host-computed agent profile for one bot. */
@Serializable
data class InsightAgentProfile(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val account: String = "",
    val accountIcon: String = "",
    val accountTone: String = "",
    val mode: String = "",
    val modeIcon: String = "",
    val modeTone: String = "",
    val fast: Boolean = false,
    val fastIcon: String = "",
    val fastTone: String = "",
    val catalog: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<InsightAgentLine> = emptyList(),
)

@Serializable data class InsightAgentLine(
    val id: String = "", val label: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val set: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed summary of a bot's chat rhythm. */
@Serializable
data class InsightConversation(
    val headline: String = "",
    val tone: String = "",
    val line: String = "",
    val total: String = "",
    val entries: Int = 0,
    val dominant: String = "",
    val workHeavy: Boolean = false,
    val userLed: Boolean = false,
    val repliesPerMessage: Int = 0,
    val stepsPerMessage: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val kinds: List<InsightConversationKind> = emptyList(),
)

@Serializable data class InsightConversationKind(
    val kind: String = "", val label: String = "", val count: Int = 0,
    val icon: String = "", val tone: String = "", val percent: Int = 0, val bar: String = "",
)

/** A host-computed summary of recent tool failures. */
@Serializable
data class InsightFailures(
    val headline: String = "",
    val tone: String = "",
    val total: String = "",
    val refusals: Int = 0,
    val rate: Int = 0,
    val network: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val categories: List<InsightFailureCategory> = emptyList(),
    val items: List<InsightFailureItem> = emptyList(),
)

@Serializable data class InsightFailureCategory(
    val kind: String = "", val count: Int = 0, val icon: String = "",
    val tone: String = "", val percent: Int = 0, val bar: String = "",
)
@Serializable data class InsightFailureItem(
    val name: String = "", val kind: String = "", val reason: String = "",
    val age: String = "", val icon: String = "", val tone: String = "",
)

/** A host-computed summary of approval cards waiting on the user. */
@Serializable
data class InsightApprovals(
    val headline: String = "",
    val tone: String = "",
    val total: String = "",
    val waiting: Int = 0,
    val urgent: Int = 0,
    val oldest: String = "",
    val bar: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val items: List<InsightApprovalItem> = emptyList(),
)

@Serializable data class InsightApprovalItem(
    val id: String = "", val title: String = "", val detail: String = "",
    val age: String = "", val icon: String = "", val tone: String = "", val urgent: Boolean = false,
)

/** A host-computed summary of the time zones a bot's routines use. */
@Serializable
data class InsightTimezone(
    val headline: String = "",
    val tone: String = "",
    val local: String = "",
    val primary: String = "",
    val zones: Int = 0,
    val schedules: String = "",
    val multi: Boolean = false,
    val remote: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val all: List<InsightZoneLine> = emptyList(),
    val remoteList: List<InsightRemoteZone> = emptyList(),
)

@Serializable data class InsightZoneLine(
    val name: String = "", val label: String = "", val icon: String = "",
    val tone: String = "", val count: Int = 0, val local: Boolean = false,
    val percent: Int = 0, val bar: String = "",
)
@Serializable data class InsightRemoteZone(val label: String = "", val count: Int = 0)

/** A host-computed executive digest for one bot. */
@Serializable
data class InsightDigest(
    val headline: String = "",
    val oneLine: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val areas: String = "",
    val onTrack: Int = 0,
    val attention: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val highlights: List<InsightDigestStat> = emptyList(),
    val stats: List<InsightDigestStat> = emptyList(),
)

@Serializable data class InsightDigestStat(
    val id: String = "", val label: String = "", val value: String = "",
    val icon: String = "", val tone: String = "",
)

/** A host-computed, prioritised suggestion list for one bot. */
@Serializable
data class InsightAdvice(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val total: String = "",
    val urgent: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val items: List<InsightAdviceItem> = emptyList(),
)

@Serializable data class InsightAdviceItem(
    val id: String = "", val title: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val priority: Int = 0,
    val priorityLabel: String = "", val priorityIcon: String = "", val priorityTone: String = "",
)

/** A host-computed session and sync state for one bot. */
@Serializable
data class InsightSession(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val session: String = "",
    val sessionIcon: String = "",
    val sessionTone: String = "",
    val revision: String = "",
    val unread: String = "",
    val bar: String = "",
    val deleted: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<InsightSessionLine> = emptyList(),
)

@Serializable data class InsightSessionLine(
    val id: String = "", val label: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val ok: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed summary of one bot's work environment. */
@Serializable
data class InsightWorkspace(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val folder: String = "",
    val folderIcon: String = "",
    val folderTone: String = "",
    val set: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<InsightWorkspaceLine> = emptyList(),
)

@Serializable data class InsightWorkspaceLine(
    val id: String = "", val label: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val set: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed memory posture for one bot. */
@Serializable
data class InsightMemory(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val shared: String = "",
    val sharedIcon: String = "",
    val sharedTone: String = "",
    val sharedDetail: String = "",
    val harness: Boolean = false,
    val harnessIcon: String = "",
    val harnessTone: String = "",
    val skills: Int = 0,
    val on: Int = 0,
    val hasAny: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<InsightMemoryLine> = emptyList(),
)

@Serializable data class InsightMemoryLine(
    val id: String = "", val title: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val on: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed summary of one bot's model tuning. */
@Serializable
data class InsightTuning(
    val headline: String = "",
    val tone: String = "",
    val count: String = "",
    val total: Int = 0,
    val on: Int = 0,
    val off: Int = 0,
    val fast: Boolean = false,
    val effort: String = "",
    val effortIcon: String = "",
    val reasoning: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val options: List<InsightTuningOption> = emptyList(),
    val enabled: List<InsightTuningEnabled> = emptyList(),
)

@Serializable data class InsightTuningOption(
    val key: String = "", val title: String = "", val value: String = "",
    val icon: String = "", val tone: String = "",
)
@Serializable data class InsightTuningEnabled(val title: String = "", val value: String = "", val icon: String = "")

/** A host-computed summary of one bot's connected accounts. */
@Serializable
data class InsightConnectors(
    val headline: String = "",
    val tone: String = "",
    val total: Int = 0,
    val distinct: Int = 0,
    val duplicates: List<String> = emptyList(),
    val chat: Boolean = false,
    val mail: Boolean = false,
    val code: Boolean = false,
    val money: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val categories: List<InsightConnectorCategory> = emptyList(),
    val connectors: List<InsightConnectorLine> = emptyList(),
)

@Serializable data class InsightConnectorCategory(
    val kind: String = "", val count: Int = 0, val icon: String = "",
    val tone: String = "", val percent: Int = 0, val bar: String = "",
)
@Serializable data class InsightConnectorLine(
    val name: String = "", val category: String = "", val icon: String = "",
    val tone: String = "", val detail: String = "",
)

/** A host-computed roster identity for one bot. */
@Serializable
data class InsightIdentity(
    val name: String = "",
    val headline: String = "",
    val tone: String = "",
    val band: String = "",
    val bandIcon: String = "",
    val bandTone: String = "",
    val section: String = "",
    val mode: String = "",
    val rank: Int = 0,
    val color: String = "",
    val colorIcon: String = "",
    val shape: String = "",
    val look: String = "",
    val visible: Boolean = false,
    val score: Int = 0,
    val count: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val traits: List<InsightIdentityTrait> = emptyList(),
)

@Serializable data class InsightIdentityTrait(
    val id: String = "", val label: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val on: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed resilience summary for one bot. */
@Serializable
data class InsightResilience(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val bar: String = "",
    val chain: String = "",
    val total: Int = 0,
    val hasChain: Boolean = false,
    val hasOverride: Boolean = false,
    val customCommands: Int = 0,
    val selfFallbacks: Boolean = false,
    val switches: List<InsightSwitch> = emptyList(),
    val summary: List<InsightSummaryRow> = emptyList(),
    val fallbacks: List<InsightFallback> = emptyList(),
)

@Serializable data class InsightSwitch(val label: String = "", val on: Boolean = false)
@Serializable data class InsightFallback(
    val backend: String = "", val model: String = "", val command: String = "",
    val label: String = "", val icon: String = "", val tone: String = "",
)

/** A host-computed trust posture for one bot. */
@Serializable
data class InsightTrust(
    val mode: String = "",
    val modeLabel: String = "",
    val modeIcon: String = "",
    val modeTone: String = "",
    val modeDetail: String = "",
    val score: Int = 0,
    val tone: String = "",
    val label: String = "",
    val headline: String = "",
    val total: Int = 0,
    val countLabel: String = "",
    val broad: Int = 0,
    val command: Int = 0,
    val hasDeny: Boolean = false,
    val hasBroadAllow: Boolean = false,
    val asksForEverything: Boolean = false,
    val decisions: List<InsightDecisionCount> = emptyList(),
    val summary: List<InsightSummaryRow> = emptyList(),
    val rules: List<InsightTrustRule> = emptyList(),
)

@Serializable data class InsightDecisionCount(val decision: String = "", val count: Int = 0)
@Serializable data class InsightTrustRule(
    val pattern: String = "", val kind: String = "", val decision: String = "",
    val icon: String = "", val tone: String = "", val broad: Boolean = false,
)

/** A host-computed summary of one bot's skill library. */
@Serializable
data class InsightLibrary(
    val headline: String = "",
    val tone: String = "",
    val count: String = "",
    val total: Int = 0,
    val distinct: Int = 0,
    val duplicates: List<String> = emptyList(),
    val hasScript: Boolean = false,
    val hasReference: Boolean = false,
    val hasAsset: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val kinds: List<InsightLibraryKind> = emptyList(),
    val skills: List<InsightLibrarySkill> = emptyList(),
)

@Serializable data class InsightLibraryKind(
    val kind: String = "", val count: Int = 0, val icon: String = "",
    val tone: String = "", val percent: Int = 0, val bar: String = "",
)
@Serializable data class InsightLibrarySkill(
    val name: String = "", val kind: String = "", val icon: String = "",
    val tone: String = "", val detail: String = "",
)

/** A host-computed summary of one bot's routines. */
@Serializable
data class InsightRoutineSummary(
    val count: String = "",
    val active: Int = 0,
    val paused: Int = 0,
    val scheduled: Int = 0,
    val activePercent: Int = 0,
    val bar: String = "",
    val next: String = "",
    val mix: String = "",
    val clock: Boolean = false,
    val event: Boolean = false,
    val allScheduled: Boolean = false,
    val headline: String = "",
    val tone: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val soonest: List<InsightSoonestRoutine> = emptyList(),
    val pausedList: List<InsightPausedRoutine> = emptyList(),
)

@Serializable data class InsightSummaryRow(val key: String = "", val value: String = "")
@Serializable data class InsightSoonestRoutine(
    val id: String = "", val name: String = "", val enabled: Boolean = false,
    val trigger: String = "", val triggerIcon: String = "", val nextIn: String = "",
    val stateIcon: String = "", val stateTone: String = "",
)
@Serializable data class InsightPausedRoutine(val id: String = "", val name: String = "")

/** A host-computed weekly summary for one bot. */
@Serializable
data class InsightWeek(
    val headline: String = "",
    val tone: String = "",
    val verdict: String = "",
    val total: Int = 0,
    val totalLabel: String = "",
    val average: Int = 0,
    val peak: InsightPeak? = null,
    val quietDays: Int = 0,
    val activeStreak: Int = 0,
    val quietStreak: Int = 0,
    val aboveAverage: Int = 0,
    val change: Int = 0,
    val perfectAttendance: Boolean = false,
    val grew: Boolean = false,
    val trend: InsightTrend = InsightTrend(),
    val top: List<InsightDayBar> = emptyList(),
    val quiet: List<InsightDayBar> = emptyList(),
    val days: List<InsightDayCount> = emptyList(),
)

@Serializable data class InsightPeak(val label: String = "", val count: Int = 0)
@Serializable data class InsightTrend(val kind: String = "", val label: String = "", val icon: String = "", val tone: String = "")
@Serializable data class InsightDayBar(val label: String = "", val count: Int = 0, val bar: String = "")
@Serializable data class InsightDayCount(val key: String = "", val label: String = "", val count: Int = 0)

/** A host-computed capability profile for one bot. */
@Serializable
data class InsightProfile(
    val headline: String = "",
    val enabled: Int = 0,
    val total: Int = 0,
    val missing: Int = 0,
    val coverage: Int = 0,
    val bar: String = "",
    val nextUp: List<InsightNextUp> = emptyList(),
    val caps: List<InsightCapability> = emptyList(),
)

@Serializable data class InsightNextUp(val title: String = "", val icon: String = "")
@Serializable data class InsightCapability(
    val id: String = "", val title: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val enabled: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed configuration health report for one bot. */
@Serializable
data class InsightHealth(
    val score: Int = 0,
    val label: String = "",
    val tone: String = "",
    val headline: String = "",
    val blocking: Boolean = false,
    val errors: Int = 0,
    val warnings: Int = 0,
    val counts: List<InsightSeverityCount> = emptyList(),
    val checks: List<InsightCheck> = emptyList(),
)

@Serializable data class InsightSeverityCount(val severity: String = "", val count: Int = 0)
@Serializable data class InsightCheck(
    val id: String = "", val title: String = "", val detail: String = "",
    val severity: String = "", val tone: String = "", val icon: String = "",
)

/** A host-computed activity timeline: folded runs, a daily strip and a summary. */
@Serializable
data class InsightActivity(
    val headline: String = "",
    val steps: Int = 0,
    val errors: Int = 0,
    val dominant: String = "",
    val span: String = "",
    val total: Int = 0,
    val counts: List<InsightCount> = emptyList(),
    val groups: List<InsightGroup> = emptyList(),
    val items: List<InsightStep> = emptyList(),
    val days: List<InsightDay> = emptyList(),
)

@Serializable data class InsightCount(val kind: String = "", val count: Int = 0, val percent: Int = 0)
@Serializable data class InsightGroup(val kind: String = "", val label: String = "", val icon: String = "", val tone: String = "", val count: Int = 0)
@Serializable data class InsightStep(val text: String = "", val detail: String = "", val status: String = "", val icon: String = "", val tone: String = "")
@Serializable data class InsightDay(val day: String = "", val label: String = "", val count: Int = 0, val width: Int = 0)

@Serializable
data class InsightRoutine(
    val id: String = "",
    val name: String = "",
    val enabled: Boolean = true,
    val schedules: List<String> = emptyList(),
    val nextIn: String? = null,
    val nextInMs: Long? = null,
)

@Serializable
data class InsightUsage(
    val used: Long = 0L,
    val usedText: String = "",
    val limit: Long? = null,
    val resetsAt: Long? = null,
)

@Serializable
data class InsightSkills(
    val total: Int = 0,
    val resources: Map<String, Int> = emptyMap(),
)
