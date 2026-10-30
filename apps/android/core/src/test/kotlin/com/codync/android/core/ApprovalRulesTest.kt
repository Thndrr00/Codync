package com.codync.android.core

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.encodeToJsonElement
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ApprovalRulesTest {
    private fun rule(pattern: String, decision: String = "allow") = ApprovalRule(pattern, null, decision)

    @Test fun `moving a rule changes who decides first and a bad move changes nothing`() {
        val rules = listOf(rule("a"), rule("b"), rule("c"))
        assertEquals(listOf("c", "a", "b"), ApprovalRules.move(rules, 2, 0).map { it.pattern })
        assertEquals(listOf("b", "a", "c"), ApprovalRules.move(rules, 0, 1).map { it.pattern })
        assertEquals(rules, ApprovalRules.move(rules, 0, 3))
        assertEquals(rules, ApprovalRules.move(rules, -1, 0))
    }

    @Test fun `saving drops blank patterns and trims the rest`() {
        val cleaned = ApprovalRules.clean(listOf(rule("  git status*  "), rule("   "), ApprovalRule("ls", "", "ask")))
        assertEquals(listOf(ApprovalRule("git status*", null, "allow"), ApprovalRule("ls", null, "ask")), cleaned)
    }

    @Test fun `the host's answer is worded from its decision`() {
        val forbidden = ApprovalEvaluation("forbidden", pattern = "rm -rf /")
        assertTrue(forbidden.describe().startsWith("Never allowed") && "rm -rf /" in forbidden.describe())
        assertEquals("Allowed by rule 2 (“git *”).", ApprovalEvaluation("rule", pattern = "git *", index = 1, decision = "allow").describe())
        assertTrue(ApprovalEvaluation("rule", pattern = "x", index = 0, decision = "deny").describe().startsWith("Denied by rule 1"))
        val setting = ApprovalEvaluation("setting", permission = "ask", chained = true, allowPassedOver = 0).describe()
        assertTrue(setting, "it asks you" in setting && "Rule 1 would allow it, but it runs several commands" in setting)
        assertTrue("approved automatically" in ApprovalEvaluation("setting", permission = "auto").describe())
    }

    @Test fun `the host's evaluation decodes, and the rules go out the way the host reads them`() {
        val json = Json.parseToJsonElement("""{"evaluation":{"outcome":"rule","index":0,"decision":"allow","pattern":"echo *","chained":false,"allowPassedOver":null}}""")
        val evaluation = ApprovalRules.decodeEvaluation(json)
        assertEquals("rule", evaluation.outcome)
        assertEquals(0, evaluation.index)
        assertNull(evaluation.allowPassedOver)
        val sent = WireJson.encodeToJsonElement(listOf(ApprovalRule("ls", null, "deny")))
        assertEquals("deny", sent.toString().substringAfter("\"decision\":\"").substringBefore("\""))
    }
}

class QuickRulesTest {
    @Test fun `a quick rule is worded by the kind of action, not by a rule number`() {
        val denied = ApprovalEvaluation("rule", pattern = "*", index = 3, decision = "deny", kind = "execute", quick = true)
        assertEquals("Denied by the quick rule for shell commands.", denied.describe())
        val files = ApprovalEvaluation("rule", pattern = "*", index = 0, decision = "allow", kind = "edit", quick = true)
        assertEquals("Allowed by the quick rule for files.", files.describe())
    }

    @Test fun `the quick classes are the ones the host enforces`() {
        assertEquals(listOf("files", "shell", "browse"), ApprovalRules.QUICK_CLASSES.map { it.first })
        assertEquals(listOf("", "allow", "ask", "block"), ApprovalRules.QUICK_POLICIES.map { it.first })
    }
}
