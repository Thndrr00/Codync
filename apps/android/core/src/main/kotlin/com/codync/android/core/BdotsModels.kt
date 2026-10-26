package com.codync.android.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonPrimitive

@Serializable
enum class ActionClass {
    @SerialName("files") FILES,
    @SerialName("shell") SHELL,
    @SerialName("browse") BROWSE,
    @SerialName("browser") BROWSER,
    @SerialName("apps") APPS,
    @SerialName("bots") BOTS,
    @SerialName("routines") ROUTINES,
}

@Serializable
enum class RulePolicy {
    @SerialName("allow") ALLOW,
    @SerialName("ask") ASK,
    @SerialName("block") BLOCK,
}

@Serializable
data class ProactiveSettings(
    val enabled: Boolean = false,
    val everyHours: Int = 4,
    val quietStart: Int = 22,
    val quietEnd: Int = 8,
    val timezone: String? = null,
    val maxPerDay: Int = 3,
    val appsInBackground: Boolean = true,
)

@Serializable
data class AppLink(
    val token: String,
    val botId: String,
    val slug: String,
    val label: String = "Shared link",
    val mode: String = "view",
    val createdAt: Long = 0L,
    val lastOpenedAt: Long? = null,
    val opens: Int = 0,
)

@Serializable
data class BotAppCollectionSummary(
    val collection: String,
    val count: Int = 0,
    val updatedAt: Long = 0L,
)

@Serializable
data class BotApp(
    val slug: String,
    val botId: String,
    val title: String,
    val description: String? = null,
    val entry: String = "index.html",
    val version: Int = 1,
    val createdAt: Long = 0L,
    val updatedAt: Long = 0L,
    val ownerToken: String? = null,
    val url: String = "",
    val links: List<AppLink> = emptyList(),
    val collections: List<BotAppCollectionSummary> = emptyList(),
)

@Serializable
data class OpenItem(
    val id: String,
    val botId: String,
    val title: String,
    val note: String = "",
    val sourceUrl: String? = null,
    val createdAt: Long = 0L,
    val updatedAt: Long = 0L,
    val checkAfter: Long? = null,
    val status: String = "open",
)

@Serializable
data class ConnectorFailure(
    val kind: String,
    val message: String,
    val at: Long = 0L,
)

/** Canonicalizes a project workspace path so Windows/Unix separator & trailing-slash variants match (N16). */
fun canonicalWorkspacePath(path: String): String {
    val normalized = path.trim().replace('\\', '/').trimEnd('/')
    if (normalized.isEmpty()) return ""
    return if (normalized.length >= 2 && normalized[1] == ':') normalized.lowercase() else normalized
}

/**
 * Returns other active bots working in the same project workspace as [bot] (N16: simultaneous project work warning).
 * Managed personal workspaces (`managedWorkspace == true` or blank `cwd`) and group chats are excluded.
 */
fun sharedWorkspacePeers(bot: Bot, roster: List<Bot>): List<Bot> {
    if (bot.kind == "group" || bot.managedWorkspace) return emptyList()
    val target = canonicalWorkspacePath(bot.cwd)
    if (target.isEmpty()) return emptyList()
    return roster.filter { peer ->
        peer.id != bot.id &&
            !peer.deleted &&
            peer.kind != "group" &&
            !peer.managedWorkspace &&
            peer.status in listOf("working", "needsInput") &&
            canonicalWorkspacePath(peer.cwd) == target
    }
}

@Serializable
data class MessageBookmark(
    val id: String,
    val botId: String,
    val entryId: String,
    val threadId: String? = null,
    val author: String = "",
    val preview: String = "",
    val note: String = "",
    val createdAt: Long = 0L,
    val updatedAt: Long = 0L,
)

@Serializable
data class PendingDecision(
    val entryId: String,
    val botId: String,
    val kind: String,
    val title: String,
    val detail: String,
    val createdAt: Long,
)

/** Collects unresolved permissions, passkeys, and error incidents across chat entries (Ficha N12). */
fun collectPendingDecisions(entries: List<Entry>): List<PendingDecision> {
    val out = mutableListOf<PendingDecision>()
    for (e in entries) {
        val status = e.status ?: "pending"
        val title = e.data["title"]?.jsonPrimitive?.contentOrNull
        val domain = e.data["domain"]?.jsonPrimitive?.contentOrNull
        val err = e.data["error"]?.jsonPrimitive?.contentOrNull
        when {
            e.kind == "permission" && status == "pending" ->
                out += PendingDecision(
                    entryId = e.id,
                    botId = e.botId,
                    kind = "permission",
                    title = title?.takeIf { it.isNotBlank() } ?: "Tool permission required",
                    detail = e.text,
                    createdAt = e.createdAt,
                )
            e.kind == "passkey" && status == "pending" ->
                out += PendingDecision(
                    entryId = e.id,
                    botId = e.botId,
                    kind = "passkey",
                    title = domain?.takeIf { it.isNotBlank() }?.let { "Passkey for $it" } ?: "Passkey verification required",
                    detail = e.text,
                    createdAt = e.createdAt,
                )
            e.kind == "error" && (!err.isNullOrBlank() || e.text.isNotBlank()) ->
                out += PendingDecision(
                    entryId = e.id,
                    botId = e.botId,
                    kind = "incident",
                    title = title?.takeIf { it.isNotBlank() } ?: "Agent incident",
                    detail = err ?: e.text,
                    createdAt = e.createdAt,
                )
        }
    }
    return out.sortedByDescending { it.createdAt }
}

@Serializable
data class DeliverableItem(
    val fileId: String,
    val entryId: String,
    val botId: String,
    val name: String,
    val size: Long,
    val sha256: String,
    val createdAt: Long,
)

/** Extracts shared files and generated deliverables from chat entries (Ficha N09). */
fun collectDeliverables(entries: List<Entry>): List<DeliverableItem> {
    val seen = mutableSetOf<String>()
    val out = mutableListOf<DeliverableItem>()
    for (e in entries) {
        for (f in e.files) {
            if (f.id.isBlank() || !seen.add(f.id)) continue
            out += DeliverableItem(
                fileId = f.id,
                entryId = e.id,
                botId = e.botId,
                name = f.name,
                size = f.size,
                sha256 = f.sha256,
                createdAt = e.createdAt,
            )
        }
    }
    return out.sortedByDescending { it.createdAt }
}

data class WorkerRoleTemplate(
    val id: String,
    val label: String,
    val summary: String,
    val standingInstructions: String,
    val avatarLook: String,
    val avatarColor: String,
    val actionRules: Map<ActionClass, RulePolicy>,
)

/** Specialized worker role profiles with preset standing instructions, avatar look & autonomy policies (Ficha N15). */
val WORKER_ROLE_TEMPLATES: List<WorkerRoleTemplate> = listOf(
    WorkerRoleTemplate(
        id = "assistant",
        label = "General Assistant",
        summary = "Balanced day-to-day coding and operational partner",
        standingInstructions = "Act as a pragmatic software partner. Explain key tradeoffs concisely, verify changes with tests, and keep the user informed of blockers.",
        avatarLook = "round",
        avatarColor = "blue",
        actionRules = mapOf(
            ActionClass.FILES to RulePolicy.ALLOW,
            ActionClass.SHELL to RulePolicy.ASK,
            ActionClass.BROWSE to RulePolicy.ALLOW,
            ActionClass.BOTS to RulePolicy.ALLOW,
        ),
    ),
    WorkerRoleTemplate(
        id = "reviewer",
        label = "Code Reviewer",
        summary = "Read-first auditor focused on regressions, security and edge cases",
        standingInstructions = "Focus on code review, correctness, security boundaries, and missing tests. Do not mutate files or run destructive commands unless explicitly asked.",
        avatarLook = "owl",
        avatarColor = "amber",
        actionRules = mapOf(
            ActionClass.FILES to RulePolicy.ASK,
            ActionClass.SHELL to RulePolicy.ASK,
            ActionClass.BROWSE to RulePolicy.ALLOW,
            ActionClass.BROWSER to RulePolicy.BLOCK,
        ),
    ),
    WorkerRoleTemplate(
        id = "coordinator",
        label = "Team Coordinator",
        summary = "Delegates tasks across specialist bots and tracks open items",
        standingInstructions = "Break complex goals into clear subtasks, delegate to specialist team bots, track open items until resolved, and synthesize concise status updates.",
        avatarLook = "visor",
        avatarColor = "purple",
        actionRules = mapOf(
            ActionClass.BOTS to RulePolicy.ALLOW,
            ActionClass.ROUTINES to RulePolicy.ALLOW,
            ActionClass.APPS to RulePolicy.ALLOW,
            ActionClass.SHELL to RulePolicy.ASK,
        ),
    ),
    WorkerRoleTemplate(
        id = "researcher",
        label = "Deep Researcher",
        summary = "Investigates documentation, web sources and codebase architecture",
        standingInstructions = "Gather primary sources, cross-check claims against code and documentation, cite concrete files and URLs, and organize findings into structured briefs.",
        avatarLook = "scout",
        avatarColor = "teal",
        actionRules = mapOf(
            ActionClass.BROWSE to RulePolicy.ALLOW,
            ActionClass.FILES to RulePolicy.ALLOW,
            ActionClass.SHELL to RulePolicy.BLOCK,
            ActionClass.BROWSER to RulePolicy.ALLOW,
        ),
    ),
    WorkerRoleTemplate(
        id = "qa",
        label = "QA Verifier",
        summary = "Runs test suites, reproduces bugs and validates user flows",
        standingInstructions = "Reproduce reported issues systematically, run unit and end-to-end checks, inspect logs, and report pass/fail evidence clearly.",
        avatarLook = "boxy",
        avatarColor = "green",
        actionRules = mapOf(
            ActionClass.FILES to RulePolicy.ALLOW,
            ActionClass.SHELL to RulePolicy.ALLOW,
            ActionClass.BROWSER to RulePolicy.ALLOW,
            ActionClass.BROWSE to RulePolicy.ALLOW,
        ),
    ),
)

private val LEADING_FILLER_REGEX = Regex("^(?:um+|uh+|eh+|erm+|este+|pues|bueno|like)\\b[,\\.\\s]*", RegexOption.IGNORE_CASE)
private val INLINE_FILLER_REGEX = Regex("\\b(?:um+|uh+|eh+|erm+)\\b[,\\.\\s]*", RegexOption.IGNORE_CASE)
private val SPACE_BEFORE_PUNCT_REGEX = Regex("\\s+([,.;:!?])")
private val NO_SPACE_AFTER_PUNCT_REGEX = Regex("([,.;:!?])([^\\s,.;:!?])")
private val MULTI_SPACE_REGEX = Regex("\\s{2,}")
private val SENTENCE_START_REGEX = Regex("([.!?]\\s+)([a-zñáéíóúü])")

/**
 * Local, deterministic voice dictation polisher inspired by Coucou (`investigacion-coucou-integracion-codync.md`):
 * strips leading/inline filler vocalizations, normalizes spacing around punctuation, and capitalizes sentences.
 */
fun polishVoiceDictation(raw: String): String {
    var text = raw.trim()
    if (text.isEmpty()) return ""
    text = text.replace(LEADING_FILLER_REGEX, "").replace(INLINE_FILLER_REGEX, " ")
    text = text
        .replace(SPACE_BEFORE_PUNCT_REGEX, "$1")
        .replace(NO_SPACE_AFTER_PUNCT_REGEX, "$1 $2")
        .replace(MULTI_SPACE_REGEX, " ")
        .trim()
    if (text.isEmpty()) return ""
    text = text.replaceFirstChar { it.uppercaseChar() }
    text = SENTENCE_START_REGEX.replace(text) { m ->
        m.groupValues[1] + m.groupValues[2].uppercase()
    }
    return text
}

