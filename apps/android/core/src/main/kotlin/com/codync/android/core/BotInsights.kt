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
    val shortcuts: InsightShortcuts = InsightShortcuts(),
    val palette: InsightPalette = InsightPalette(),
    val picker: InsightPicker = InsightPicker(),
    val contextselect: InsightContextSelect = InsightContextSelect(),
    val mdtables: InsightMdTables = InsightMdTables(),
    val contextpin: InsightContextPin = InsightContextPin(),
    val verify: InsightVerify = InsightVerify(),
    val autofill: InsightAutofill = InsightAutofill(),
    val rrule: InsightRrule = InsightRrule(),
    val iteration: InsightIteration = InsightIteration(),
    val approvalrules: InsightApprovalRules = InsightApprovalRules(),
    val brainpicker: InsightBrainPicker = InsightBrainPicker(),
    val engine: InsightEngine = InsightEngine(),
    val accounts: InsightAccounts = InsightAccounts(),
    val plans: InsightPlans = InsightPlans(),
    val providers: InsightProviders = InsightProviders(),
    val avatar: InsightAvatar = InsightAvatar(),
    val secretcleanup: InsightSecretCleanup = InsightSecretCleanup(),
    val toolreplay: InsightToolReplay = InsightToolReplay(),
    val vault: InsightVault = InsightVault(),
    val devices: InsightDevices = InsightDevices(),
    val noai: InsightNoAi = InsightNoAi(),
    val missions: InsightMissions = InsightMissions(),
    val cmdpolicy: InsightCmdPolicy = InsightCmdPolicy(),
    val router: InsightRouter = InsightRouter(),
    val modelprefs: InsightModelPrefs = InsightModelPrefs(),
    val botstore: InsightBotStore = InsightBotStore(),
    val promptactions: InsightPromptActions = InsightPromptActions(),
    val csiterm: InsightCsiTerm = InsightCsiTerm(),
    val plugins: InsightPlugins = InsightPlugins(),
    val service: InsightService = InsightService(),
    val draftstash: InsightDraftStash = InsightDraftStash(),
    val layout: InsightLayout = InsightLayout(),
    val channelaccess: InsightChannelAccess = InsightChannelAccess(),
    val work: InsightWork = InsightWork(),
    val contextblocks: InsightContextBlocks = InsightContextBlocks(),
    val toollabels: InsightToolLabels = InsightToolLabels(),
    val mail: InsightMail = InsightMail(),
    val media: InsightMedia = InsightMedia(),
    val replypolicy: InsightReplyPolicy = InsightReplyPolicy(),
    val notices: InsightNotices = InsightNotices(),
    val github: InsightGithub = InsightGithub(),
    val aurora: InsightAurora = InsightAurora(),
    val whatsapp: InsightWhatsapp = InsightWhatsapp(),
    val coucoumail: InsightCoucouMail = InsightCoucouMail(),
    val pills: InsightPills = InsightPills(),
    val coucoushortcuts: InsightCoucouShortcuts = InsightCoucouShortcuts(),
    val islandlayout: InsightIslandLayout = InsightIslandLayout(),
    val recap: InsightRecap = InsightRecap(),
    val whatsnew: InsightWhatsNew = InsightWhatsNew(),
    val mochidesktop: InsightMochiDesktop = InsightMochiDesktop(),
    val spotify: InsightSpotify = InsightSpotify(),
    val anim: InsightAnim = InsightAnim(),
    val closed: InsightClosed = InsightClosed(),
    val plan: InsightPlan = InsightPlan(),
    val diff: InsightDiff = InsightDiff(),
    val routines: List<InsightRoutine> = emptyList(),
    val usage: InsightUsage = InsightUsage(),
    val skills: InsightSkills = InsightSkills(),
)

/** A host-computed file diff summary. */
@Serializable
data class InsightDiff(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val kinds: Int = 0, val tools: Int = 0, val path: String = "",
    val added: Int = 0, val removed: Int = 0, val step: String = "",
    val parsed: Int = 0, val preview: String = "", val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed plan usage summary. */
@Serializable
data class InsightPlan(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val titles: Int = 0, val windows: Int = 0, val claude: String = "",
    val codex: String = "", val color: String = "", val age: String = "",
    val reset: String = "", val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed closed-island summary. */
@Serializable
data class InsightClosed(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val styles: Int = 0, val height: Int = 0, val minWidth: Int = 0, val maxWidth: Int = 0,
    val dot: String? = null, val claude: String = "", val music: String = "",
    val note: Boolean = false, val width: Int = 0, val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed animation summary. */
@Serializable
data class InsightAnim(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val eases: Int = 0, val modes: Int = 0, val closeMs: Int = 0,
    val curve: String = "", val spring: String = "", val response: Double = 0.0,
    val damping: Double = 0.0, val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed Spotify summary. */
@Serializable
data class InsightSpotify(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val playing: Boolean = false, val position: String = "", val remaining: String = "",
    val progress: String = "", val volume: String = "", val track: String = "",
    val dance: Boolean = false, val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed Mochi desktop summary. */
@Serializable
data class InsightMochiDesktop(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val phases: Int = 0, val panel: Int = 0, val sleep: Int = 0, val distance: Int = 0,
    val gaze: String = "", val alert: Boolean = false, val pointerMs: Int = 0,
    val line: String = "", val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed what's-new summary. */
@Serializable
data class InsightWhatsNew(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val icons: Int = 0, val stats: Int = 0, val version: String = "",
    val title: String = "", val items: Int = 0, val iconName: String = "",
    val laterMs: Int = 0, val page: String = "", val hint: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed weekly recap summary. */
@Serializable
data class InsightRecap(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val hour: Int = 0, val monday: String = "", val previous: String = "",
    val name: String = "", val seconds: Int = 0, val duration: String = "",
    val range: String = "", val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed island layout summary. */
@Serializable
data class InsightIslandLayout(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val modes: Int = 0, val states: Int = 0, val emotes: Int = 0,
    val panel: String = "", val notch: String = "", val corner: Int = 0,
    val glow: String = "", val opacity: Double = 0.0, val project: String = "",
    val wash: String = "", val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed shortcuts summary. */
@Serializable
data class InsightCoucouShortcuts(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val modifiers: Int = 0, val keys: String = "", val normalized: String? = null,
    val recorded: String = "", val clashes: Int = 0, val next: Int? = null,
    val pill: String? = null, val action: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed pill catalog summary. */
@Serializable
data class InsightPills(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val catalog: Int = 0, val available: Int = 0, val main: String = "",
    val declared: Int = 0, val subtitle: String = "", val soon: Boolean = false,
    val first: String = "", val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed Mail pill summary. */
@Serializable
data class InsightCoucouMail(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val id: String = "", val filters: Int = 0, val fields: Int = 0,
    val name: String = "", val messageId: String = "", val rule: String = "",
    val reason: String = "", val line: String = "", val plans: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed WhatsApp summary. */
@Serializable
data class InsightWhatsapp(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val green: String = "", val states: Int = 0, val errors: Int = 0, val steps: Int = 0,
    val initials: String = "", val duration: String = "", val preview: String = "",
    val ago: String = "", val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed aurora background summary. */
@Serializable
data class InsightAurora(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val ground: String = "", val palette: Int = 0, val pitch: Int = 0,
    val rows: Int = 0, val brightness: String = "", val fallsRight: Boolean = false,
    val bayer: Int = 0, val sample: String = "", val fieldVar: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed GitHub CI summary. */
@Serializable
data class InsightGithubCi(val failing: Boolean = false, val color: String = "", val value: String = "")

/** A host-computed GitHub alert. */
@Serializable
data class InsightGithubAlert(val kind: String = "", val text: String = "", val id: String = "")

/** A host-computed GitHub pulse summary. */
@Serializable
data class InsightGithub(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val sections: Int = 0, val prs: String = "", val ci: InsightGithubCi = InsightGithubCi(),
    val label: String = "", val repo: String = "", val url: String? = null,
    val day: String = "", val alert: InsightGithubAlert? = null,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed mail summary. */
@Serializable
data class InsightMail(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val choice: String = "", val addresses: List<String> = emptyList(), val valid: List<String> = emptyList(),
    val level: String = "", val cap: Int = 0, val auto: Boolean = false, val errors: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed media summary. */
@Serializable
data class InsightMedia(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val moods: Int = 0, val agentMoods: Int = 0, val categories: List<String> = emptyList(),
    val tool: Boolean = false, val rating: String = "", val stickers: String = "",
    val after: Boolean = false, val line: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed reply policy summary. */
@Serializable
data class InsightReplyPolicy(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val verdict: String = "", val silent: Boolean = false, val mustAnswer: Boolean = false,
    val sentinel: Boolean = false, val visible: String = "", val reason: String = "",
    val lead: Int = 0, val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed notices summary. */
@Serializable
data class InsightNotices(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val kinds: Int = 0, val quiet: Int = 0, val agentOnly: Int = 0, val text: String = "",
    val publish: Boolean = false, val priority: Int = 0, val handoff: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed tool labels summary. */
@Serializable
data class InsightToolLabels(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val key: String = "", val app: String? = null, val label: String? = null,
    val labelPt: String? = null, val humanized: String = "", val matched: Boolean = false,
    val technical: Boolean = false, val subject: String = "", val phrases: String = "",
    val line: String = "", val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed context blocks summary. */
@Serializable
data class InsightContextBlocks(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val blocks: Int = 0, val turns: Int = 0, val found: Int = 0,
    val noReply: Boolean = false, val reaction: Boolean = false,
    val folded: List<String> = emptyList(), val conversation: String = "",
    val stable: Boolean = false, val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed work header summary. */
@Serializable
data class InsightWorkHeader(
    val steps: String = "", val connectors: String? = null, val duration: String? = null,
)

/** A host-computed work summary. */
@Serializable
data class InsightWork(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val steps: Int = 0, val calls: Int = 0, val folded: Int = 0,
    val duration: String = "", val state: String = "", val apps: Int = 0,
    val quiet: Boolean = false, val header: InsightWorkHeader = InsightWorkHeader(),
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed channel access summary. */
@Serializable
data class InsightChannelAccess(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val channels: Int = 0, val summaryLine: String = "", val hasAdmins: Boolean = false,
    val pairing: Boolean = false, val approved: Int = 0, val provider: String = "",
    val creator: String = "", val api: String = "", val complete: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed pane layout summary. */
@Serializable
data class InsightLayout(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val panes: Int = 0, val groups: Int = 0, val depth: Int = 0,
    val orientation: String = "", val drop: String = "", val normalized: Boolean = false,
    val active: String = "", val ids: List<String> = emptyList(),
    val weights: List<Double> = emptyList(), val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed composer draft stash summary. */
@Serializable
data class InsightDraftStash(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val key: String = "", val newKey: String = "", val stamp: Int = 0,
    val entries: Int = 0, val drafts: Int = 0, val cleared: String = "",
    val merged: Boolean = false, val title: String = "", val compacted: Int = 0,
    val legacy: Boolean = false, val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed service launch summary. */
@Serializable
data class InsightService(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val taskName: String = "", val runKey: String = "", val unit: String = "",
    val modes: Int = 0, val platforms: String = "", val parsed: List<String> = emptyList(),
    val pathMac: String = "", val pathLinux: String = "", val drift: Boolean = false,
    val quote: String = "", val running: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed plugin store summary. */
@Serializable
data class InsightPlugins(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val root: String = "", val staging: String = "", val settings: String = "",
    val validId: Boolean = false, val reserved: Boolean = false, val toolName: Boolean = false,
    val relativePath: Boolean = false, val manifestVersion: Int = 0, val apiVersion: String = "",
    val compatible: Boolean = false, val problem: String? = null, val staleLock: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed CSI terminal summary. */
@Serializable
data class InsightCsiTerm(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val prefix: String = "", val cursorHome: String = "", val eraseLine: String = "",
    val eraseScreen: String = "", val cursorStyle: String = "", val paramByte: Boolean = false,
    val intermediateByte: Boolean = false, val finalByte: Boolean = false,
    val move: String = "", val eraseLines: String = "", val scroll: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed prompt action summary. */
@Serializable
data class InsightPromptActions(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val idCandidate: Boolean = false, val busyRetryMs: Int = 0, val retryIntervalMs: Int = 0,
    val ttlMs: Int = 0, val timeout: Boolean = false, val notOwned: Boolean = false,
    val surface: String? = null, val forwarded: Boolean = false, val inFlight: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed bot store summary. */
@Serializable
data class InsightBotStore(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val id: String = "", val name: String = "", val folder: String = "",
    val deliverable: String = "", val valid: Boolean = false, val errors: List<String> = emptyList(),
    val schedules: Int = 0, val fleet: Boolean = false, val dangerous: Boolean = false,
    val dangerousTools: Int = 0, val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed model preferences summary. */
@Serializable
data class InsightModelPrefs(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val levels: List<String> = emptyList(), val openaiLevels: List<String> = emptyList(),
    val anthropicLevels: List<String> = emptyList(), val modelDefault: String = "",
    val chat: String? = null, val adaptive: Boolean = false, val label: String = "",
    val humanized: String = "", val openrouter: Boolean = false, val prefs: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed router threshold. */
@Serializable
data class InsightRouterThreshold(val name: String = "", val value: Double = 0.0)

/** A host-computed router summary. */
@Serializable
data class InsightRouter(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val thresholds: List<InsightRouterThreshold> = emptyList(), val quietMs: Int = 0,
    val addressee: Boolean = false, val selected: List<String> = emptyList(),
    val everyone: Boolean = false, val answered: Boolean = false,
    val scores: List<String> = emptyList(), val social: String? = null,
    val strategy: String = "", val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed command policy summary. */
@Serializable
data class InsightCmdPolicy(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val words: Int = 0, val commands: Int = 0, val verdict: String = "",
    val denied: Boolean = false, val secretPath: Boolean = false, val readOnly: Boolean = false,
    val repo: String? = null, val pushes: Int = 0, val inline: Boolean = false,
    val denyKinds: Int = 0, val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed mission counts summary. */
@Serializable
data class InsightMissionCounts(
    val total: Int = 0, val done: Int = 0, val doing: Int = 0, val waiting: Int = 0,
)

/** A host-computed mission waiting part. */
@Serializable
data class InsightMissionWaiting(val key: String = "", val count: Int = 0)

/** A host-computed mission composer command. */
@Serializable
data class InsightMissionCommand(val command: String = "", val arg: String = "")

/** A host-computed mission pull request. */
@Serializable
data class InsightMissionPr(val number: String = "", val url: String? = null)

/** A host-computed missions summary. */
@Serializable
data class InsightMissions(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val cells: List<String> = emptyList(), val counts: InsightMissionCounts = InsightMissionCounts(),
    val open: Boolean = false, val waiting: List<InsightMissionWaiting> = emptyList(),
    val tick: Int = 0, val command: InsightMissionCommand? = null, val pr: InsightMissionPr? = null,
    val span: String = "", val commands: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed no-AI card. */
@Serializable
data class InsightNoAiCard(
    val agentId: String = "", val forUserId: String? = null, val intent: String = "",
    val provider: String? = null, val autoOpen: Boolean? = null, val own: String? = null,
    val signedOut: Boolean? = null, val planLine: String? = null, val model: String? = null,
    val why: String? = null, val missing: String? = null, val held: Boolean? = null,
)

/** A host-computed no-AI summary. */
@Serializable
data class InsightNoAi(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val card: InsightNoAiCard = InsightNoAiCard(), val line: String = "",
    val state: String = "", val passes: Boolean = false, val opens: Boolean = false,
    val brain: String = "", val kind: String = "", val choices: Int = 0,
    val firstChoice: String = "", val autoOpen: Boolean? = null,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed device portrait. */
@Serializable
data class InsightPortrait(val kind: String = "", val state: String = "")

/** A host-computed device stub in a connect stage. */
@Serializable
data class InsightDeviceStub(val id: String? = null, val name: String? = null)

/** A host-computed Android connect stage. */
@Serializable
data class InsightDeviceStage(val kind: String = "", val phone: InsightDeviceStub? = null)

/** A host-computed Android cable guide. */
@Serializable
data class InsightDeviceSteps(
    val states: Map<String, String> = emptyMap(), val now: String = "",
    val open: String = "", val byHand: List<String> = emptyList(),
)

/** A host-computed devices summary. */
@Serializable
data class InsightDevices(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val devices: Int = 0, val phones: Int = 0, val all: Int = 0, val look: String = "",
    val portrait: InsightPortrait = InsightPortrait(), val line: String = "",
    val rows: Int = 0, val link: String = "", val stage: InsightDeviceStage = InsightDeviceStage(),
    val steps: InsightDeviceSteps = InsightDeviceSteps(),
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed card summary inside the vault. */
@Serializable
data class InsightCardSummary(
    val brand: String = "", val last4: String = "", val expMonth: Int = 0,
    val expYear: Int = 0, val name: String? = null,
)

/** A host-computed parsed card expiry. */
@Serializable
data class InsightCardExpiry(val expMonth: Int = 0, val expYear: Int = 0)

/** A host-computed bot vault summary. */
@Serializable
data class InsightVault(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val authTypes: Int = 0, val siteless: Boolean = false, val variable: String = "",
    val problem: String? = null, val code: String? = null, val identifier: Boolean = false,
    val otp: Boolean = false, val card: InsightCardSummary = InsightCardSummary(),
    val title: String = "", val expiry: InsightCardExpiry? = null, val policy: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed tool replay summary. */
@Serializable
data class InsightToolReplay(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val calls: Int = 0, val kept: Int = 0, val budget: Int = 0, val left: Int = 0,
    val replay: Boolean = false, val followUp: Boolean = false,
    val credentialField: Boolean = false, val secretTool: Boolean = false,
    val outcome: String = "", val sample: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed secret cleanup summary. */
@Serializable
data class InsightSecretCleanup(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val reference: String = "", val kind: String = "", val prefix: String? = null,
    val known: Boolean = false, val attemptMs: Int = 0, val backupMs: Int = 0,
    val storeMs: Int = 0, val providerMs: Int = 0, val retryDue: Int = 0,
    val backupDue: Int = 0, val expired: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed avatar color summary. */
@Serializable
data class InsightAvatar(
    val headline: String = "", val tone: String = "", val color: String = "",
    val hex: String = "", val rgb: String = "", val eye: String = "",
    val ink: String = "", val bucket: String = "", val contrast: String = "",
    val invites: Int = 0, val initials: String = "", val background: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed model provider summary. */
@Serializable
data class InsightProviders(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val providers: Int = 0, val keys: Int = 0, val models: Int = 0, val usable: Int = 0,
    val guess: String = "", val label: String = "", val site: String = "",
    val hasPlans: Boolean = false, val billing: Boolean = false, val privacy: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed coding plan summary. */
@Serializable
data class InsightPlans(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val plan: String = "", val providerPlan: String? = null, val models: Int = 0,
    val choice: String = "", val freeHint: Boolean = false, val keyPlan: Boolean = false,
    val placeholder: String = "", val geminiPersonal: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed AI accounts summary. */
@Serializable
data class InsightAccounts(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val accounts: Int = 0, val plans: Int = 0, val state: String = "",
    val offersAgain: Boolean = false, val title: String = "", val action: String = "",
    val actionIcon: String = "", val lendable: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed brain engine summary. */
@Serializable
data class InsightEngine(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val engine: String = "", val state: String = "", val off: Boolean = false,
    val executor: String = "", val line: String = "", val modelBrain: Boolean = false,
    val auto: Boolean = false, val computers: List<String> = emptyList(),
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed brain picker summary. */
@Serializable
data class InsightBrainPicker(
    val headline: String = "", val tone: String = "", val description: String = "",
    val groups: List<String> = emptyList(), val free: Int = 0, val keywords: String = "",
    val auto: Boolean = false, val connect: Boolean = false, val label: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed approval rules summary. */
@Serializable
data class InsightApprovalRules(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val tool: String = "", val sqlRead: Boolean = false, val target: String = "",
    val command: Boolean = false, val maxActions: Int = 0, val actions: Int = 0,
    val rules: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed iteration bookkeeping summary. */
@Serializable
data class InsightIteration(
    val headline: String = "", val tone: String = "", val used: Int = 0, val max: Int = 0,
    val warning: Boolean = false, val warningText: String = "", val ghosts: Int = 0,
    val tools: Int = 0, val toolTail: Boolean = false, val steerAfter: Int? = null,
    val restartLabel: String = "", val restartIcon: String = "", val refunded: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed recurrence rule summary. */
@Serializable
data class InsightRrule(
    val headline: String = "", val tone: String = "", val freq: String = "",
    val interval: Int = 0, val times: List<String> = emptyList(), val weekdays: List<String> = emptyList(),
    val cron: String? = null, val cronCompatible: Boolean = false, val workdays: Boolean = false,
    val error: String = "", val timezone: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed autofill classification summary. */
@Serializable
data class InsightAutofill(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val controls: Int = 0, val otp: Int = 0, val checkout: Int = 0, val best: String = "",
    val tokens: Int = 0, val paymentTokens: Int = 0, val addressTokens: Int = 0,
    val password: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed verification-on-stop summary. */
@Serializable
data class InsightVerify(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val paths: Int = 0, val dirs: Int = 0, val proseOnly: Boolean = false,
    val enabled: Boolean = false, val evidence: String = "", val nudge: Boolean = false,
    val nudgeLabel: String = "", val maxPaths: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed context window pin summary. */
@Serializable
data class InsightContextPin(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val window: Int = 0, val marker: String = "", val suffix: String = "",
    val pinned: Boolean = false, val fallback: Boolean = false, val pinnedSuffix: String = "",
    val advertised: Int? = null, val disagrees: Boolean = false, val known: Boolean = false,
    val ratio: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed markdown table summary. */
@Serializable
data class InsightMdTables(
    val headline: String = "", val tone: String = "", val rows: Int = 0,
    val cols: Int = 0, val wide: Int = 0, val narrow: Boolean = false,
    val layout: String = "", val divider: String = "", val aligned: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed context budget summary. */
@Serializable
data class InsightContextSelect(
    val headline: String = "", val tone: String = "", val estimate: Int = 0,
    val available: Int = 0, val fixed: Int = 0, val window: Int = 0, val output: Int = 0,
    val fits: Boolean = false, val tooSmall: Boolean = false, val strategy: String = "",
    val cache: String = "", val cacheIcon: String = "", val kept: Int = 0,
    val omitted: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed model-picker usage summary. */
@Serializable
data class InsightPicker(
    val headline: String = "", val tone: String = "", val bar: String = "",
    val used: String = "", val left: String = "", val style: String = "",
    val resets: String = "", val featured: List<String> = emptyList(),
    val gauges: Int = 0, val levels: Int = 0, val width: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed command palette summary. */
@Serializable
data class InsightPalette(
    val headline: String = "", val tone: String = "", val icon: String = "",
    val command: String = "", val needle: String = "", val results: Int = 0,
    val recentPages: Int = 0, val commands: Int = 0, val bots: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
)

/** A host-computed keyboard shortcut summary. */
@Serializable
data class InsightShortcuts(
    val headline: String = "", val tone: String = "", val modifier: String = "",
    val quote: String = "", val commandMenu: String? = null, val primary: Boolean = false,
    val browserShift: Boolean = false, val resolved: List<InsightShortcutRow> = emptyList(),
    val summary: List<InsightSummaryRow> = emptyList(),
)

@Serializable
data class InsightShortcutRow(
    val command: String = "", val label: String = "", val icon: String = "", val tone: String = "",
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
