package com.codync.android

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.design.*
import com.kyant.shapes.Capsule
import kotlin.math.roundToInt

@Composable internal fun BotConstellation(
    bots: List<Bot>,
    open: (Bot) -> Unit,
    actions: (Bot) -> List<MenuItem>,
    modifier: Modifier = Modifier,
    totalCount: Int = bots.size,
    onClearFilters: (() -> Unit)? = null,
    reselectToken: Int = 0,
) {
    val reduced = reduceMotion()
    val scroll = rememberScrollState()
    var handledReselect by remember { mutableIntStateOf(reselectToken) }
    LaunchedEffect(reselectToken) {
        if (reselectToken != handledReselect) {
            handledReselect = reselectToken
            scroll.animateScrollTo(0)
        }
    }
    val phase = rememberConstellationPhase(!reduced && bots.any { it.status == "working" })
    // A stable clock snapshot keeps sizes from drifting whenever unrelated app state changes.
    val now = remember(bots) { System.currentTimeMillis() }
    Column(modifier.fillMaxSize()) {
        if (bots.size < totalCount && onClearFilters != null) {
            Row(
                Modifier.padding(horizontal = 16.dp, vertical = 4.dp)
                    .clip(Capsule())
                    .background(Palette.surface)
                    .clickable(role = Role.Button, onClick = onClearFilters)
                    .padding(horizontal = 12.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text("Showing ${bots.size} of $totalCount bots", style = Type.caption.medium, color = Palette.secondary)
                Text("Clear", style = Type.caption.semibold, color = Palette.accent)
            }
        }
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth().semantics { isTraversalGroup = true }) {
            val width = maxWidth.value.coerceAtLeast(ConstellationLayout.CELL_WIDTH)
            val viewport = maxHeight.value
            val localDensity = LocalDensity.current
            val cellHeight = maxOf(ConstellationLayout.CELL_HEIGHT, 112f + 48f * localDensity.fontScale)
            val layout = remember(bots.size, width, viewport, cellHeight) { ConstellationLayout.place(bots.size, width, viewport, cellHeight) }
            val density = localDensity.density
            Box(Modifier.fillMaxSize().verticalScroll(scroll)) {
                Box(Modifier.fillMaxWidth().height(layout.height.dp)) {
                    bots.forEachIndexed { index, bot ->
                        key(bot.id) {
                            val point = layout.points[index]
                            val spec = if (reduced) snap<Float>() else Motion.layout()
                            val x = animateFloatAsState(point.x * density, spec, label = "Orb x")
                            val y = animateFloatAsState(point.y * density, spec, label = "Orb y")
                            val visible by remember(point, viewport, density, cellHeight) {
                                derivedStateOf {
                                    val top = scroll.value / density
                                    point.y + cellHeight >= top && point.y <= top + viewport
                                }
                            }
                            val diameter = ConstellationLayout.diameter(bot.lastAt, bot.unread, now)
                            Column(
                                Modifier.offset { IntOffset(x.value.roundToInt(), y.value.roundToInt()) }
                                    .width(ConstellationLayout.CELL_WIDTH.dp)
                                    .contextActions(onClick = { open(bot) }, items = { actions(bot) })
                                    .semantics(mergeDescendants = true) {
                                        contentDescription = "${bot.name}, ${constellationStatus(bot.status)}, ${bot.unread.coerceAtLeast(0)} unread"
                                        traversalIndex = index.toFloat()
                                    },
                                horizontalAlignment = Alignment.CenterHorizontally,
                            ) {
                                ConstellationOrb(
                                    bot.avatarShape,
                                    bot.avatarColor,
                                    bot.status,
                                    bot.unread,
                                    diameter.dp,
                                    phase,
                                    moving = visible && !reduced,
                                )
                                Text(
                                    bot.name,
                                    Modifier.fillMaxWidth().clearAndSetSemantics {},
                                    style = Type.subheadline.medium,
                                    color = Palette.text,
                                    textAlign = TextAlign.Center,
                                    maxLines = 2,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

internal fun constellationStatus(status: String): String = when (status) {
    "working" -> "Working"
    "needsInput" -> "Needs you"
    "error" -> "Error"
    else -> "Idle"
}

@Composable internal fun RosterPresentationHeader(
    bots: List<Bot>,
    selection: RosterPresentation,
    select: (RosterPresentation) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        SegmentedChoice(selection, listOf(RosterPresentation.List to "Lista", RosterPresentation.Constellation to "Constelación"), select)
        val needsInput = bots.count { it.status == "needsInput" }
        Text(
            "${bots.count { it.status == "working" }} trabajando · $needsInput ${if (needsInput == 1) "te necesita" else "te necesitan"}",
            style = Type.subheadline,
            color = Palette.secondary,
        )
    }
}
