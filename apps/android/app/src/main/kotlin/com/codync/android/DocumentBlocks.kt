package com.codync.android

/** Markdown snippets only: these never create or execute host resources. */
internal data class DocumentBlock(val command: String, val label: String, val markdown: String)

internal val documentBlocks = listOf(
    DocumentBlock("doc", "Document section", "## Document\n\n"),
    DocumentBlock("routine", "Routine outline", "## Routine\n\n- Trigger: \n- Steps: \n"),
    DocumentBlock("skill", "Skill outline", "## Skill\n\n### Purpose\n\n### Instructions\n\n"),
    DocumentBlock("task", "Task checkbox", "- [ ] Task\n"),
    DocumentBlock("heading", "Heading", "## "),
    DocumentBlock("list", "Bullet list", "- "),
    DocumentBlock("code", "Code block", "```\n\n```\n"),
    DocumentBlock("quote", "Quote", "> "),
)

private val documentSlashQuery = Regex("/[a-z]*")
private val documentFence = Regex("^ {0,3}(`{3,}|~{3,})(.*)")

// FieldBox exposes text rather than selection, so mobile completion uses the final line.
internal fun documentSlash(body: String): Pair<Int, List<DocumentBlock>>? {
    val start = body.lastIndexOf('\n') + 1
    val query = body.substring(start)
    if (!documentSlashQuery.matches(query)) return null
    var marker: Char? = null
    var size = 0
    for (line in body.substring(0, start).lineSequence()) {
        val match = documentFence.find(line.trimEnd('\r')) ?: continue
        val fence = match.groupValues[1]
        if (marker == null) { marker = fence.first(); size = fence.length }
        else if (marker == fence.first() && fence.length >= size && match.groupValues[2].isBlank()) marker = null
    }
    if (marker != null) return null
    val items = documentBlocks.filter { it.command.startsWith(query.drop(1)) }
    return if (items.isEmpty()) null else start to items
}

internal fun documentTree(items: List<BotDocument>, searching: Boolean): List<Pair<BotDocument, Int>> {
    val known = items.map { it.id }.toSet()
    val children = items.groupBy { if (searching || it.parentId !in known) null else it.parentId }
    val seen = mutableSetOf<String>()
    val rows = mutableListOf<Pair<BotDocument, Int>>()
    fun visit(page: BotDocument, depth: Int) {
        val pending = java.util.ArrayDeque<Pair<BotDocument, Int>>()
        pending.addLast(page to depth)
        while (pending.isNotEmpty()) {
            val (current, level) = pending.removeLast()
            if (!seen.add(current.id)) continue
            rows += current to level
            for (child in children[current.id].orEmpty().asReversed()) pending.addLast(child to level + 1)
        }
    }
    for (page in children[null].orEmpty()) visit(page, 0)
    for (page in items) if (page.id !in seen) visit(page, 0)
    return rows
}

internal fun documentDescendants(items: List<BotDocument>, id: String?): Set<String> {
    val found = mutableSetOf(id ?: return emptySet())
    var changed: Boolean
    do {
        val count = found.size
        for (page in items) if (page.parentId in found) found += page.id
        changed = count != found.size
    } while (changed)
    return found
}
