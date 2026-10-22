package com.codync.android.core

/**
 * One thing an alert asked this phone to do without opening the app: reply in the bot's chat, or answer a
 * pending approval. It names the account context, computer and chat it belongs to, so an action that outlives
 * any of them is dropped instead of reaching another conversation.
 */
data class NotificationAction(
    val nonce: String,
    val contextId: String,
    val computerId: String,
    val botId: String,
    /** The reply's text; null for an approval. */
    val reply: String? = null,
    /** The pending permission card and the chosen option; null for a reply. */
    val entryId: String? = null,
    val optionId: String? = null,
) {
    val isApproval: Boolean get() = entryId != null
}

/** Validates and names the actions a notification sends: one account, one computer, one chat, one thing to do. */
object NotificationActionPlan {
    const val MAX_REPLY = 4_000
    const val WORK_PREFIX = "codync-notification-"
    private val TOKEN = Regex("[A-Za-z0-9_-]{1,128}")
    private val NONCE = Regex("[A-Za-z0-9-]{8,64}")
    private val OPTION = Regex("[A-Za-z0-9_.:-]{1,128}")

    /** A reply to [botId]'s main chat; null when the text is empty, too long or the target is malformed. */
    fun reply(nonce: String, contextId: String, computerId: String, botId: String, text: String): NotificationAction? {
        val trimmed = text.trim()
        if (trimmed.isEmpty() || trimmed.length > MAX_REPLY) return null
        return base(nonce, contextId, computerId, botId)?.copy(reply = trimmed)
    }

    /** An answer to [entryId] with one of the agent's options; null when either id is malformed. */
    fun approval(
        nonce: String, contextId: String, computerId: String, botId: String, entryId: String, optionId: String,
    ): NotificationAction? {
        if (!validEntry(entryId) || !validOption(optionId)) return null
        return base(nonce, contextId, computerId, botId)?.copy(entryId = entryId, optionId = optionId)
    }

    /** Work with the same nonce is the same action: a repeated tap never sends twice. */
    fun workName(nonce: String): String = WORK_PREFIX + nonce

    fun freshNonce(): String = java.util.UUID.randomUUID().toString()

    /** The pending card an alert may name: the same check `approval` applies to a tap. */
    fun validEntry(entryId: String): Boolean = TOKEN.matches(entryId)

    /** One of the agent's option ids, as an alert may show it. */
    fun validOption(optionId: String): Boolean = OPTION.matches(optionId)

    private fun base(nonce: String, contextId: String, computerId: String, botId: String): NotificationAction? {
        if (!NONCE.matches(nonce) || !TOKEN.matches(contextId) || !TOKEN.matches(computerId) || !TOKEN.matches(botId)) return null
        return NotificationAction(nonce, contextId, computerId, botId)
    }
}
