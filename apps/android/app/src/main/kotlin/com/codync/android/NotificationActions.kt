package com.codync.android

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.app.RemoteInput
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.Data
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.codync.android.core.EncryptedChannel
import com.codync.android.core.HostConnector
import com.codync.android.core.HostError
import com.codync.android.core.NotificationAction
import com.codync.android.core.NotificationActionPlan
import com.codync.android.core.ShareIntake
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/**
 * A reply or approval the user gave from an alert. The alert disappears at once; the work is durable, so a
 * locked screen, a dead process or a lost connection doesn't lose it, and the host's idempotent nonce means a
 * repeated tap never sends twice. Nothing is reported as done until the computer answered.
 */
class NotificationActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val nonce = intent.getStringExtra(EXTRA_NONCE) ?: return
        val scope = intent.getStringExtra(EXTRA_SCOPE) ?: return
        val computer = intent.getStringExtra(EXTRA_COMPUTER) ?: return
        val bot = intent.getStringExtra(EXTRA_BOT) ?: return
        val action = when (intent.action) {
            ACTION_REPLY -> NotificationActionPlan.reply(
                nonce, scope, computer, bot,
                RemoteInput.getResultsFromIntent(intent)?.getCharSequence(REPLY_KEY)?.toString().orEmpty(),
            )
            ACTION_APPROVE -> NotificationActionPlan.approval(
                nonce, scope, computer, bot,
                intent.getStringExtra(EXTRA_ENTRY).orEmpty(), intent.getStringExtra(EXTRA_OPTION).orEmpty(),
            )
            else -> null
        } ?: return
        NotificationDisplay.cancelAlert(context, scope, computer, bot)
        NotificationActionWork.enqueue(context, action)
    }

    companion object {
        const val ACTION_REPLY = "com.codync.android.action.NOTIFICATION_REPLY"
        const val ACTION_APPROVE = "com.codync.android.action.NOTIFICATION_APPROVE"
        const val REPLY_KEY = "reply"
        const val EXTRA_NONCE = "nonce"
        const val EXTRA_SCOPE = "scope"
        const val EXTRA_COMPUTER = "computer"
        const val EXTRA_BOT = "bot"
        const val EXTRA_ENTRY = "entry"
        const val EXTRA_OPTION = "option"
    }
}

/** What the alert's actions hand to the worker: its own action, never a generic receiver. */
internal object NotificationActionWork {
    private const val KIND = "kind"
    private const val REPLY = "reply"
    private const val APPROVE = "approve"

    fun enqueue(context: Context, action: NotificationAction) {
        val request = OneTimeWorkRequestBuilder<NotificationActionWorker>()
            .setInputData(data(action))
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 15, TimeUnit.SECONDS)
            .build()
        // The same nonce is the same action: a repeated tap replaces nothing and sends nothing twice.
        WorkManager.getInstance(context).enqueueUniqueWork(NotificationActionPlan.workName(action.nonce), ExistingWorkPolicy.KEEP, request)
    }

    fun action(data: Data): NotificationAction? = when (data.getString(KIND)) {
        REPLY -> NotificationActionPlan.reply(
            data.getString(NONCE).orEmpty(), data.getString(SCOPE).orEmpty(), data.getString(COMPUTER).orEmpty(),
            data.getString(BOT).orEmpty(), data.getString(MESSAGE).orEmpty(),
        )
        APPROVE -> NotificationActionPlan.approval(
            data.getString(NONCE).orEmpty(), data.getString(SCOPE).orEmpty(), data.getString(COMPUTER).orEmpty(),
            data.getString(BOT).orEmpty(), data.getString(ENTRY).orEmpty(), data.getString(OPTION).orEmpty(),
        )
        else -> null
    }

    private fun data(action: NotificationAction): Data = Data.Builder()
        .putString(KIND, if (action.isApproval) APPROVE else REPLY)
        .putString(NONCE, action.nonce)
        .putString(SCOPE, action.contextId)
        .putString(COMPUTER, action.computerId)
        .putString(BOT, action.botId)
        .putString(MESSAGE, action.reply)
        .putString(ENTRY, action.entryId)
        .putString(OPTION, action.optionId)
        .build()

    private const val NONCE = "nonce"
    private const val SCOPE = "scope"
    private const val COMPUTER = "computer"
    private const val BOT = "bot"
    private const val MESSAGE = "message"
    private const val ENTRY = "entry"
    private const val OPTION = "option"
}

/** Sends one notification action to the computer that owns it, or says it couldn't. */
class NotificationActionWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result {
        val action = NotificationActionWork.action(inputData) ?: return Result.failure()
        if (PushPreferences.retired(applicationContext)) return Result.success()
        // The alert belonged to the account that was active then; a replaced account never answers for it, and
        // there is nobody left to tell in that account's shade.
        val user = PushPreferences.activeUser(applicationContext) ?: return Result.failure()
        if (LocalStore.referenceId(user) != action.contextId) return Result.failure()
        val local = LocalStore(applicationContext, user)
        val computer = withContext(Dispatchers.IO) { local.computers() }.firstOrNull { it.id == action.computerId } ?: return fail(action, local)
        if (!local.hasIdentity()) return fail(action, local)
        var channel: EncryptedChannel? = null
        return try {
            withTimeout(45_000) {
                val identity = withContext(Dispatchers.IO) { local.identity() }
                val connected = HostConnector.connect(computer, identity)
                channel = connected
                connected.state.firstReady()
                if (action.isApproval) {
                    connected.call("respondPermission", buildJsonObject {
                        put("botId", action.botId); put("entryId", action.entryId.orEmpty()); put("optionId", action.optionId.orEmpty())
                    })
                } else {
                    connected.call("send", buildJsonObject {
                        put("botId", action.botId); put("text", action.reply.orEmpty()); put("clientNonce", action.nonce)
                    })
                }
            }
            Result.success()
        } catch (cancelled: CancellationException) {
            if (cancelled is TimeoutCancellationException) retry(action, local) else throw cancelled
        } catch (error: HostError) {
            // The computer refused for good (unpaired, incompatible or the request is gone): asking again won't help.
            if (error.status in listOf(403, 426)) fail(action, local) else retry(action, local)
        } catch (error: Exception) {
            retry(action, local)
        } finally {
            channel?.close()
        }
    }

    private suspend fun retry(action: NotificationAction, local: LocalStore?): Result =
        if (runAttemptCount < 4) Result.retry() else fail(action, local)

    /**
     * Says what really happened; never claims the message or the answer arrived. A reply that never left the
     * phone waits in the bot's draft, so "Open Codync" can send it instead of asking the user to type it again.
     */
    private suspend fun fail(action: NotificationAction, local: LocalStore?): Result {
        if (!action.isApproval && local != null) withContext(Dispatchers.IO) { preserveDraft(local, action) }
        val text = if (action.isApproval) "Your answer wasn't sent. Tap to review the request."
            else "Your reply wasn't sent. Tap to send it from the chat."
        NotificationDisplay.actionFailed(applicationContext, action.contextId, action.computerId, action.botId, text, action.reply)
        return Result.failure()
    }

    private fun preserveDraft(local: LocalStore, action: NotificationAction) {
        try {
            val book = DraftBook(local.directory, action.computerId)
            val drafts = book.read()
            val lane = ComposerDraft.lane(action.botId, null)
            val current = drafts.firstOrNull { it.lane == lane } ?: ComposerDraft(action.botId)
            val merged = current.copy(text = ShareIntake.merge(current.text, action.reply.orEmpty()))
            book.save(drafts.filter { it.lane != lane } + merged)
        } catch (_: Exception) {
            // The failure notice still tells the truth; the text just isn't kept.
        }
    }
}
