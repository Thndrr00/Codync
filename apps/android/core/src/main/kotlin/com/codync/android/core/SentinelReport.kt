package com.codync.android.core

import kotlinx.serialization.Serializable

/** The host's read-only scan of recent runtime failures (`sentinelHealth`). */
@Serializable data class SentinelReport(
    val scannedAt: Long,
    val lookbackMs: Long,
    val entriesExamined: Long,
    val issues: List<SentinelIssue>,
) {
    val healthy: Boolean get() = issues.isEmpty()
}

/** Kind and severity remain strings so newer host classifications stay readable. */
@Serializable data class SentinelIssue(
    val signature: String,
    val kind: String,
    val severity: String,
    val title: String,
    val toolName: String? = null,
    val botId: String,
    val firstSeenAt: Long,
    val lastSeenAt: Long,
    val count: Long,
    val wastedTurns: Long,
    val evidenceEntryId: String,
)
