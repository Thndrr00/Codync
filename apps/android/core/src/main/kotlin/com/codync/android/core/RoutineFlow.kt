package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlin.math.exp
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToLong

// Routine flows (host/src/routines/flow/, docs/features/routine-flows.md). The host owns the rules: it validates every change
// and runs the flow. This file shapes what the screens show — node positions (saved, else automatic columns), edges, the list
// that mirrors the canvas — and pre-checks the connect rules so a drop the host would refuse never leaves the phone.
// The desktop app has the same model in `lib/routine-flow.ts`. Media nodes follow OpenBot 0.70.0
// `packages/ui/src/features/workflows/WorkflowImageNode.tsx` (a step that makes a file is a node
// like any other, linked to the steps around it).

/** [model] and [modelOptions] are what this step runs on instead of the target bot's own model; null is the bot's own. */
@Serializable data class FlowLink(
    val id: String, val routineId: String, val fromBotId: String, val toBotId: String,
    val instruction: String = "", val model: String? = null, val modelOptions: Map<String, String> = emptyMap(),
    val createdAt: Long = 0,
)

/** `status`: running, succeeded, failed, skipped or cancelled. */
@Serializable data class FlowStep(
    val id: String, val runId: String, val botId: String, val deliveryId: String? = null,
    val input: String = "", val output: String? = null, val status: String, val error: String? = null,
    val createdAt: Long = 0, val updatedAt: Long = 0,
) {
    val label: String get() = flowStatusLabel(status)
}

fun flowStatusLabel(status: String?): String = when (status) {
    "running" -> "Running"; "succeeded" -> "Succeeded"; "failed" -> "Failed"; "skipped" -> "Skipped"
    "cancelled" -> "Cancelled"; else -> "Not run yet"
}

/** A picture or clip step of a routine's flow (`host/src/store/flow_media.rs`); its node key is `media:<id>`. */
@Serializable data class FlowMedia(
    val id: String, val kind: String, val model: String, val prompt: String = "", val aspect: String = "1:1",
    /** Seconds; only a video uses it. */
    val duration: Int = 0, /** Only a video uses it. */ val resolution: String = "",
    val transparent: Boolean = false, val voice: String? = null, val createdAt: Long = 0, val routineId: String,
)

@Serializable data class FlowPosition(val nodeKey: String, val x: Double, val y: Double)

@Serializable data class FlowRoutineCard(
    val routine: Routine, val recentRuns: List<RoutineRun> = emptyList(),
    val upcomingRuns: List<Long> = emptyList(), val steps: List<FlowStep> = emptyList(),
) {
    fun step(botId: String): FlowStep? = steps.firstOrNull { it.botId == botId }
}

@Serializable data class FlowCanvas(
    val botId: String, val rev: Long = 0, val routines: List<FlowRoutineCard> = emptyList(),
    val links: List<FlowLink> = emptyList(), val positions: List<FlowPosition> = emptyList(),
    val placedBotIds: List<String> = emptyList(),
    /** The picture and clip steps of these routines (`host/src/store/flow_media.rs`). */
    val mediaNodes: List<FlowMedia> = emptyList(),
)

object RoutineFlow {
    /** The host's limit on how far a flow reaches from its routine's bot. */
    const val DEFAULT_MAX_DEPTH = 5
    const val MAX_COORDINATE = 1_000_000.0
    const val NODE_WIDTH = 220.0
    const val NODE_HEIGHT = 120.0
    const val MIN_SCALE = 0.3f
    const val MAX_SCALE = 2f
    private const val COLUMN_GAP = 90.0
    private const val ROW_GAP = 40.0
    private const val MAX_ROWS = 200

    fun routineKey(id: String) = "routine:$id"
    fun botKey(id: String) = "bot:$id"
    fun mediaKey(id: String) = "media:$id"

    /** The canvas key of a link's end, which names a bot or a `media:` step. */
    fun flowKey(id: String) = if (id.startsWith("media:")) mediaKey(id.removePrefix("media:")) else botKey(id)

    /** The media step a link end names, if it is one. */
    fun mediaEnd(id: String): String? = if (id.startsWith("media:")) id.removePrefix("media:") else null

    /** The media step of this canvas a node id stands for. */
    fun mediaOf(canvas: FlowCanvas, id: String): FlowMedia? =
        mediaEnd(id)?.let { own -> canvas.mediaNodes.firstOrNull { it.id == own } }

    /** What a media node is called: its own description, else its kind. */
    fun mediaTitle(media: FlowMedia?): String {
        val prompt = media?.prompt?.trim().orEmpty()
        return prompt.ifEmpty { if (media?.kind == "video") "Video" else "Picture" }
    }

    fun clampCoordinate(value: Double): Double = value.coerceIn(-MAX_COORDINATE, MAX_COORDINATE).roundToLong().toDouble()

    fun owner(canvas: FlowCanvas, routineId: String): String? = canvas.routines.firstOrNull { it.routine.id == routineId }?.routine?.botId

    /** How many hops from the routine's own bot each bot of its flow is (the longest path); absent when not reached. */
    fun depths(owner: String, links: List<FlowLink>): Map<String, Int> {
        val depth = mutableMapOf(owner to 0)
        // The host refuses cycles, so a depth only grows, at most once per link; the round cap keeps a bad canvas from spinning.
        repeat(links.size + 1) {
            var moved = false
            for (link in links) {
                val from = depth[link.fromBotId] ?: continue
                if ((depth[link.toBotId] ?: -1) >= from + 1) continue
                depth[link.toBotId] = from + 1
                moved = true
            }
            if (!moved) return depth
        }
        return depth
    }

    private fun reaches(links: List<FlowLink>, start: String, target: String): Boolean {
        val seen = mutableSetOf<String>()
        val queue = ArrayDeque(listOf(start))
        while (queue.isNotEmpty()) {
            val next = queue.removeLast()
            if (next == target) return true
            if (!seen.add(next)) continue
            links.filter { it.fromBotId == next }.forEach { queue.add(it.toBotId) }
        }
        return false
    }

    private fun longestFrom(links: List<FlowLink>, bot: String, seen: Set<String> = emptySet()): Int {
        if (bot in seen) return 0
        return links.filter { it.fromBotId == bot }.maxOfOrNull { 1 + longestFrom(links, it.toBotId, seen + bot) } ?: 0
    }

    /** Why `from` can't hand a routine's reply to `to` (the host's rules, in its words), or null when it can. */
    fun connectProblem(canvas: FlowCanvas, routineId: String, from: String, to: String, maxDepth: Int = DEFAULT_MAX_DEPTH): String? {
        val owner = owner(canvas, routineId) ?: return "That routine is not on this canvas."
        val links = canvas.links.filter { it.routineId == routineId }
        if (from == to) return "A bot can't hand work to itself."
        if (to == owner) return "The routine's own bot starts the flow, so nothing can hand work back to it."
        val media = mediaOf(canvas, to)
        if (media != null && media.routineId != routineId) return "That step is not part of this routine."
        val here = depths(owner, links)[from] ?: return "The flow doesn't reach that bot yet: connect it from the routine's bot first."
        if (links.any { it.fromBotId == from && it.toBotId == to }) return "These two bots are already connected in this routine."
        if (reaches(links, to, from)) return "That link would make the flow loop."
        if (here + 1 + longestFrom(links, to) > maxDepth) return "A flow reaches at most $maxDepth bots deep."
        return null
    }

    /** The bots a canvas shows: its own, every routine's bot, every link's bot ends and the bots placed by hand. */
    fun canvasBotIds(canvas: FlowCanvas): List<String> = buildList {
        add(canvas.botId)
        fun addBot(id: String) { if (!id.startsWith("media:")) add(id) }
        canvas.routines.forEach { addBot(it.routine.botId) }
        canvas.links.forEach { addBot(it.fromBotId); addBot(it.toBotId) }
        addAll(canvas.placedBotIds)
    }.distinct()

    /** The bots a canvas shows, without the media steps (`canvasBotIds`). */
    fun botIds(canvas: FlowCanvas): List<String> = canvasBotIds(canvas)

    enum class NodeKind { ROUTINE, BOT, MEDIA }

    data class Node(val key: String, val kind: NodeKind, val id: String, val x: Double, val y: Double, val saved: Boolean) {
        val isRoutine: Boolean get() = kind == NodeKind.ROUTINE
    }

    /**
     * Every node with its place: the saved one, else an automatic column. Routines stand in the first column; a bot or a
     * media step goes one column right of the deepest routine flow that reaches it, nodes no flow reaches in the last
     * column.
     */
    fun layout(canvas: FlowCanvas): List<Node> {
        val saved = canvas.positions.associateBy { it.nodeKey }
        val column = mutableMapOf<String, Int>()
        for (card in canvas.routines) {
            for ((bot, depth) in depths(card.routine.botId, canvas.links.filter { it.routineId == card.routine.id })) {
                column[bot] = max(column[bot] ?: 0, depth + 1)
            }
        }
        val lastColumn = max(1, column.values.maxOrNull() ?: 1) + 1
        val rows = mutableMapOf<Int, Int>()
        fun place(key: String, kind: NodeKind, id: String, col: Int): Node {
            val position = saved[key]
            val row = rows[col] ?: 0
            rows[col] = row + 1
            return Node(key, kind, id, position?.x ?: (col * (NODE_WIDTH + COLUMN_GAP)),
                position?.y ?: (row * (NODE_HEIGHT + ROW_GAP)), position != null)
        }
        val nodes = canvas.routines.map { place(routineKey(it.routine.id), NodeKind.ROUTINE, it.routine.id, 0) }.toMutableList()
        for (id in canvasBotIds(canvas)) nodes += place(botKey(id), NodeKind.BOT, id, column[id] ?: lastColumn)
        for (media in canvas.mediaNodes) {
            val key = mediaKey(media.id)
            nodes += place(key, NodeKind.MEDIA, key, column[key] ?: lastColumn)
        }
        return nodes
    }

    /** `linkId` is null for a routine's fixed edge to its own bot. */
    data class Edge(val id: String, val from: String, val to: String, val linkId: String?)

    fun edges(canvas: FlowCanvas): List<Edge> =
        canvas.routines.map { Edge("start:${it.routine.id}", routineKey(it.routine.id), botKey(it.routine.botId), null) } +
            canvas.links.map { Edge(it.id, flowKey(it.fromBotId), flowKey(it.toBotId), it.id) }

    /** The latest step the canvas holds for a bot, across its routines. */
    fun latestStep(canvas: FlowCanvas, botId: String): FlowStep? =
        canvas.routines.flatMap { it.steps }.filter { it.botId == botId }.maxByOrNull { it.updatedAt }

    fun isUnconnected(canvas: FlowCanvas, routineId: String): Boolean = canvas.links.none { it.routineId == routineId }

    /** One line of the list that mirrors the canvas: a routine's bot, or a bot or media step under the step that hands to it. */
    data class Row(
        val key: String, val routineId: String,
        /** The bot id, or the `media:` step id. */
        val nodeId: String, /** `bot` or `media`. */ val kind: String, val media: FlowMedia?,
        val depth: Int, val link: FlowLink?, val step: FlowStep?,
    ) {
        val botId: String get() = nodeId
    }

    /** The canvas as a list, routine by routine and depth first: what TalkBack and small screens walk. */
    fun rows(canvas: FlowCanvas): List<Row> {
        val rows = mutableListOf<Row>()
        for (card in canvas.routines) {
            val links = canvas.links.filter { it.routineId == card.routine.id }
            fun walk(nodeId: String, depth: Int, link: FlowLink?, path: Set<String>) {
                if (rows.size >= MAX_ROWS || nodeId in path) return
                val media = mediaOf(canvas, nodeId)
                rows += Row("${card.routine.id}/${link?.id ?: "start"}", card.routine.id, nodeId,
                    if (media != null) "media" else "bot", media, depth, link, card.step(nodeId))
                links.filter { it.fromBotId == nodeId }.forEach { walk(it.toBotId, depth + 1, it, path + nodeId) }
            }
            walk(card.routine.botId, 0, null, emptySet())
        }
        return rows
    }

    // MARK: view (pan and zoom)

    data class View(val x: Float = 0f, val y: Float = 0f, val scale: Float = 1f)

    /** Zoom by `factor` keeping the point (`px`, `py`) of the viewport still. */
    fun zoomAt(view: View, factor: Float, px: Float, py: Float): View {
        val scale = (view.scale * factor).coerceIn(MIN_SCALE, MAX_SCALE)
        val k = scale / view.scale
        return View(px - (px - view.x) * k, py - (py - view.y) * k, scale)
    }

    /** The view that shows every node inside a viewport of `width` × `height` pixels, with a margin. */
    fun fit(nodes: List<Node>, width: Float, height: Float, margin: Float = 24f): View {
        if (nodes.isEmpty() || width <= 0f || height <= 0f) return View()
        val left = nodes.minOf { it.x }
        val top = nodes.minOf { it.y }
        val right = nodes.maxOf { it.x + NODE_WIDTH }
        val bottom = nodes.maxOf { it.y + NODE_HEIGHT }
        val scale = min((width - 2 * margin) / (right - left).toFloat(), (height - 2 * margin) / (bottom - top).toFloat())
            .coerceIn(MIN_SCALE, 1f)
        return View((width - (right - left).toFloat() * scale) / 2 - left.toFloat() * scale,
            (height - (bottom - top).toFloat() * scale) / 2 - top.toFloat() * scale, scale)
    }

    /** The zoom factor of a pinch whose fingers moved from `before` to `after` pixels apart. */
    fun pinchFactor(before: Float, after: Float): Float = if (before <= 0f) 1f else exp((after - before) / before)

    /** The node whose card holds the canvas point, if any: where a dragged connection was dropped. */
    fun nodeAt(nodes: List<Node>, x: Double, y: Double): Node? =
        nodes.firstOrNull { x >= it.x && x <= it.x + NODE_WIDTH && y >= it.y && y <= it.y + NODE_HEIGHT }
}
