package com.codync.android

import java.util.UUID
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** OpenBot 010: one id per branch click, so a double-click replays the first branch on the host. */
internal fun branchRequestId(): String = "branch-" + UUID.randomUUID()

/** Sends the branch intent with its request id; the host owns the task and its dedupe receipt. */
fun AppStore.branchTask(botId: String, taskId: String, name: String, before: Long? = null) = action("task-branch:$taskId") {
    hostQuery("taskBranch", buildJsonObject {
        put("botId", botId)
        put("taskId", taskId)
        put("name", name)
        put("requestId", branchRequestId())
        before?.let { put("before", it) }
    })
    Unit
}
