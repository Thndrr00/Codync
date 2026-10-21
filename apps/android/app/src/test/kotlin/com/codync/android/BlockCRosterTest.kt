package com.codync.android

import android.content.SharedPreferences
import com.codync.android.core.Bot
import com.codync.android.core.Entry
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class BlockCRosterTest {

    private val bots = listOf(
        Bot(
            id = "b1",
            name = "Backend Reviewer",
            backend = "claude",
            cwd = "/Users/dev/codync/host",
            status = "working",
            startedAt = 1_700_000_000L,
            pinned = true,
            position = 0,
            lastAt = 100L,
            fastMode = true,
            fastModeSupport = "offered",
        ),
        Bot(
            id = "b2",
            name = "Android UI Fixer",
            backend = "codex",
            cwd = "/Users/dev/codync/apps/android",
            status = "needsInput",
            unread = 3,
            position = 1,
            lastAt = 200L,
            fastMode = true,
            fastModeSupport = "refused",
            fastModeIssue = "rate limit",
        ),
        Bot(
            id = "b3",
            name = "Release Squad",
            kind = "group",
            members = listOf("b1", "b2"),
            cwd = "/should/not/show",
            backend = "claude",
            position = 2,
            lastAt = 150L,
        ),
        Bot(
            id = "b4",
            name = "Personal Notes",
            backend = "gemini",
            managedWorkspace = true,
            position = 3,
            lastAt = 90L,
        ),
    )

    @Test
    fun `Idea 17 and 18 filterRosterBots preserves host order and separates statuses`() {
        val ordered = roster(bots)
        assertEquals(listOf("b1", "b2", "b3", "b4"), ordered.map(Bot::id))

        assertEquals(listOf("b2"), filterRosterBots(ordered, RosterFilter.NeedsInput).map(Bot::id))
        assertEquals(listOf("b1"), filterRosterBots(ordered, RosterFilter.Working).map(Bot::id))
        assertEquals(listOf("b2"), filterRosterBots(ordered, RosterFilter.Unread).map(Bot::id))
        assertEquals(listOf("b3"), filterRosterBots(ordered, RosterFilter.Groups).map(Bot::id))

        // Query matches folder path, agent name, or bot name while preserving order.
        assertEquals(listOf("b1", "b2"), filterRosterBots(ordered, RosterFilter.All, "codync").map(Bot::id))
        assertEquals(listOf("b4"), filterRosterBots(ordered, RosterFilter.All, "Personal workspace").map(Bot::id))
    }

    @Test
    fun `Idea 17 searchCachedMessages finds main and thread hits and marks partial cache`() {
        val entries = listOf(
            Entry(
                id = "e1",
                seq = 1L,
                botId = "b1",
                threadId = null,
                rev = 1L,
                kind = "agent",
                createdAt = 1_000L,
                data = buildJsonObject {
                    put("text", "Fixed the SQLite migration lock in MirrorDatabase")
                    put("final", true)
                },
            ),
            Entry(
                id = "e2",
                seq = 2L,
                botId = "b2",
                threadId = "root-99",
                rev = 2L,
                kind = "user",
                createdAt = 2_000L,
                data = buildJsonObject {
                    put("text", "Please check the SQLite connection pool in thread")
                },
            ),
        )

        val result = searchCachedMessages(
            bots = roster(bots),
            entries = entries,
            historyComplete = setOf("b1"),
            online = true,
            query = "SQLite",
            requestId = 42L,
        )

        assertEquals(42L, result.requestId)
        assertTrue(result.isPartialCache)
        assertEquals(2, result.messageHits.size)
        // Newest hit first, preserving threadId for jumping to the exact thread.
        assertEquals("e2", result.messageHits[0].entryId)
        assertEquals("root-99", result.messageHits[0].threadId)
        assertEquals("e1", result.messageHits[1].entryId)
        assertNull(result.messageHits[1].threadId)
    }

    @Test
    fun `Idea 20 resolveAttentionItems requires single complete pending permission for direct approval`() {
        val permissionEntry = Entry(
            id = "perm-1",
            seq = 10L,
            botId = "b2",
            rev = 10L,
            kind = "permission",
            data = buildJsonObject {
                put("status", "pending")
                put("toolKind", "execute")
                put("title", "./gradlew :app:testDebugUnitTest")
                put(
                    "options",
                    JsonArray(
                        listOf(
                            buildJsonObject {
                                put("optionId", "opt-allow")
                                put("name", "Allow once")
                                put("kind", "allow_once")
                            },
                            buildJsonObject {
                                put("optionId", "opt-deny")
                                put("name", "Deny")
                                put("kind", "reject_once")
                            },
                        ),
                    ),
                )
            },
        )

        // 1. With a single valid permission entry -> DirectPermission
        val direct = resolveAttentionItems(roster(bots), listOf(permissionEntry), emptySet())
        assertEquals(1, direct.size)
        val item = direct.first() as BotAttentionItem.DirectPermission
        assertEquals("b2", item.bot.id)
        assertEquals("Wants to run a command", item.headline)
        assertEquals("./gradlew :app:testDebugUnitTest", item.title)
        assertEquals("opt-allow", item.allowOptionId)
        assertEquals("opt-deny", item.denyOptionId)

        // 2. Without cached permission entries -> OpenChatRequired (never approves blindly)
        val fallback = resolveAttentionItems(roster(bots), emptyList(), emptySet())
        assertEquals(1, fallback.size)
        assertTrue(fallback.first() is BotAttentionItem.OpenChatRequired)

        // 3. With multiple pending permission entries -> OpenChatRequired
        val secondPermission = permissionEntry.copy(id = "perm-2", seq = 11L)
        val multiple = resolveAttentionItems(roster(bots), listOf(permissionEntry, secondPermission), emptySet())
        assertEquals(1, multiple.size)
        val multiItem = multiple.first() as BotAttentionItem.OpenChatRequired
        assertEquals(2, multiItem.pendingCount)
    }

    @Test
    fun `Idea 21 resolveBotRowMetadata distinguishes Fast states and never leaks group cwd`() {
        val m1 = resolveBotRowMetadata(bots[0])
        assertEquals("Claude Code", m1.agentLabel)
        assertEquals("host", m1.workspaceLabel)
        assertEquals(FastBadgeState.Active, m1.fastBadge)

        val m2 = resolveBotRowMetadata(bots[1])
        assertEquals("Codex", m2.agentLabel)
        assertEquals("android", m2.workspaceLabel)
        assertEquals(FastBadgeState.Refused, m2.fastBadge)

        val groupMeta = resolveBotRowMetadata(bots[2])
        assertNull(groupMeta.agentLabel)
        assertNull(groupMeta.workspaceLabel)
        assertEquals(FastBadgeState.None, groupMeta.fastBadge)
        assertEquals("2 bots", groupMeta.groupMembersLabel)
    }

    @Test
    fun `Idea 22 groupBotsByWorkspace keeps identically named folders separate and persists collapse per context`() {
        val sameFolderBots = listOf(
            Bot(id = "a", name = "Service A", cwd = "/work/client-alpha/api", position = 0),
            Bot(id = "b", name = "Service B", cwd = "/work/client-beta/api", position = 1),
            Bot(id = "g", name = "Team Chat", kind = "group", members = listOf("a", "b"), position = 2),
            Bot(id = "u", name = "Scratch Bot", cwd = "", position = 3),
        )

        val sections = groupBotsByWorkspace(sameFolderBots, computerId = "comp-1", computerName = "Work Mac")
        assertEquals(4, sections.size)
        // Two folders named "api" under different paths do not merge and disambiguate via subtitle.
        assertEquals("api", sections[0].title)
        assertEquals("Work Mac · /work/client-alpha/api", sections[0].subtitle)
        assertEquals("api", sections[1].title)
        assertEquals("Work Mac · /work/client-beta/api", sections[1].subtitle)
        assertEquals("Group chats", sections[2].title)
        assertEquals("Other bots", sections[3].title)

        val prefs = RosterPreferences(InMemorySharedPreferences())
        val key = sections[0].key
        assertFalse(prefs.isSectionCollapsed("ctx-1", key))
        prefs.toggleSectionCollapsed("ctx-1", key)
        assertTrue(prefs.isSectionCollapsed("ctx-1", key))
        assertFalse(prefs.isSectionCollapsed("ctx-2", key))
    }

    @Test
    fun `Idea 23 formatWorkingDuration handles seconds and millis and returns null when idle or missing`() {
        val workingSec = bots[0] // startedAt = 1_700_000_000L (seconds)
        assertEquals("1:25", formatWorkingDuration(workingSec, 1_700_000_085_000L))
        assertEquals("1:01:05", formatWorkingDuration(workingSec, 1_700_003_665_000L))

        val workingMs = workingSec.copy(startedAt = 1_700_000_000_000L)
        assertEquals("0:42", formatWorkingDuration(workingMs, 1_700_000_042_000L))

        // Missing startedAt or non-working status returns null
        assertNull(formatWorkingDuration(workingSec.copy(startedAt = null), 1_700_000_085_000L))
        assertNull(formatWorkingDuration(workingSec.copy(status = "idle"), 1_700_000_085_000L))
    }

    @Test
    fun `Idea 24 resolvePeekMessages returns up to 3 latest messages and preserves threadId`() {
        val entries = (1..5).map { idx ->
            Entry(
                id = "m$idx",
                seq = idx.toLong(),
                botId = "b1",
                threadId = if (idx == 5) "thread-1" else null,
                rev = idx.toLong(),
                kind = if (idx % 2 == 0) "user" else "agent",
                createdAt = idx * 100L,
                data = buildJsonObject {
                    put("text", "**Message $idx** body")
                    put("final", JsonPrimitive(true))
                },
            )
        }

        val peek = resolvePeekMessages(bots[0], entries, limit = 3)
        assertEquals(3, peek.size)
        assertEquals(listOf("m3", "m4", "m5"), peek.map(PeekMessage::id))
        assertEquals("Message 5 body", peek[2].text)
        assertEquals("thread-1", peek[2].threadId)
        assertNull(peek[0].threadId)
    }
}

private class InMemorySharedPreferences : SharedPreferences {
    private val map = mutableMapOf<String, Any?>()

    override fun getAll(): MutableMap<String, *> = map.toMutableMap()
    override fun getString(key: String?, defValue: String?): String? = map[key] as? String ?: defValue

    @Suppress("UNCHECKED_CAST")
    override fun getStringSet(key: String?, defValues: MutableSet<String>?): MutableSet<String>? =
        (map[key] as? Set<String>)?.toMutableSet() ?: defValues

    override fun getInt(key: String?, defValue: Int): Int = map[key] as? Int ?: defValue
    override fun getLong(key: String?, defValue: Long): Long = map[key] as? Long ?: defValue
    override fun getFloat(key: String?, defValue: Float): Float = map[key] as? Float ?: defValue
    override fun getBoolean(key: String?, defValue: Boolean): Boolean = map[key] as? Boolean ?: defValue
    override fun contains(key: String?): Boolean = map.containsKey(key)

    override fun edit(): SharedPreferences.Editor = object : SharedPreferences.Editor {
        private val pending = mutableMapOf<String, Any?>()
        private var clearAll = false

        override fun putString(key: String?, value: String?): SharedPreferences.Editor = apply {
            if (key != null) pending[key] = value
        }

        override fun putStringSet(key: String?, values: MutableSet<String>?): SharedPreferences.Editor = apply {
            if (key != null) pending[key] = values?.toSet()
        }

        override fun putInt(key: String?, value: Int): SharedPreferences.Editor = apply {
            if (key != null) pending[key] = value
        }

        override fun putLong(key: String?, value: Long): SharedPreferences.Editor = apply {
            if (key != null) pending[key] = value
        }

        override fun putFloat(key: String?, value: Float): SharedPreferences.Editor = apply {
            if (key != null) pending[key] = value
        }

        override fun putBoolean(key: String?, value: Boolean): SharedPreferences.Editor = apply {
            if (key != null) pending[key] = value
        }

        override fun remove(key: String?): SharedPreferences.Editor = apply {
            if (key != null) pending[key] = null
        }

        override fun clear(): SharedPreferences.Editor = apply { clearAll = true }

        override fun commit(): Boolean {
            apply()
            return true
        }

        override fun apply() {
            if (clearAll) map.clear()
            for ((k, v) in pending) {
                if (v == null) map.remove(k) else map[k] = v
            }
        }
    }

    override fun registerOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) = Unit
    override fun unregisterOnSharedPreferenceChangeListener(listener: SharedPreferences.OnSharedPreferenceChangeListener?) = Unit
}
