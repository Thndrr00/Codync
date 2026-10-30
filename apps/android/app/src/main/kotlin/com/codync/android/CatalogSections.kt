package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.codync.android.core.BotCatalog
import com.codync.android.core.BotDraft
import com.codync.android.core.BotProfile
import com.codync.android.core.TeamProfile
import com.codync.android.core.from
import com.codync.android.design.*

/** A catalog entry the sheet shows. */
private sealed interface CatalogItem {
    val id: String
    data class Bot(val profile: BotProfile) : CatalogItem { override val id get() = profile.id }
    data class Team(val team: TeamProfile) : CatalogItem { override val id get() = team.id }
}

/**
 * Extensions' ready-made bots and coordinator teams (desktop `CatalogSections`). Adding a bot creates it on the
 * computer's defaults; adding a team creates its coordinator, its teammates and their group (`installTeam`).
 */
@Composable internal fun CatalogSections(state: AppState, store: AppStore, query: String, inset: Modifier) {
    val catalog = rememberCatalog(state, store) ?: return
    var added by remember(state.computer?.id) { mutableStateOf(emptySet<String>()) }
    var open by remember { mutableStateOf<CatalogItem?>(null) }
    val matches = { name: String, summary: String -> query.isEmpty() || name.contains(query, true) || summary.contains(query, true) }
    val bots = catalog.bots.filter { matches(it.name, it.summary) }
    val teams = catalog.teams.filter { matches(it.name, it.summary) }
    val add = { item: CatalogItem ->
        val done = { added = added + item.id }
        when (item) {
            is CatalogItem.Bot -> store.addProfile(withKnownAgent(BotDraft().from(item.profile), state), item.id, done)
            is CatalogItem.Team -> store.installTeam(item.id, done)
        }
    }
    @Composable fun row(item: CatalogItem, title: String, subtitle: String, icon: @Composable () -> Unit) =
        MarketRow(title, subtitle, item.id in added, busy = catalogKey(item.id) in state.busy, open = { open = item }, icon = icon) { add(item) }

    if (bots.isNotEmpty()) Box(inset) {
        MarketSection("Bots") {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                for (p in bots) row(CatalogItem.Bot(p), p.name, p.summary) { ProfileFace(p, 46.dp) }
            }
        }
    }
    if (teams.isNotEmpty()) Box(inset) {
        MarketSection("Coordinator teams") {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                for (t in teams) row(CatalogItem.Team(t), t.name, t.summary) { TeamFace(catalog, t) }
            }
        }
    }
    CodyncSheet(open, { open = null }) { item ->
        CatalogSheet(item, catalog, item.id in added, catalogKey(item.id) in state.busy) { add(item) }
    }
}

@Composable private fun ProfileFace(profile: BotProfile, size: Dp) = CharacterAvatar(profile.avatarShape, profile.avatarColor, size)

/** The coordinator in front, its first teammate peeking behind. */
@Composable private fun TeamFace(catalog: BotCatalog, team: TeamProfile) {
    val profiles = catalog.profiles(team)
    Box(Modifier.size(46.dp), contentAlignment = Alignment.BottomEnd) {
        profiles.getOrNull(1)?.let { Box(Modifier.align(Alignment.TopStart)) { ProfileFace(it, 28.dp) } }
        Box(Modifier.offset(x = 2.dp)) { ProfileFace(profiles.first(), 34.dp) }
    }
}

/** A ready-made bot's or team's details: its instructions, or who leads and who is on the team, with Add. */
@Composable private fun CatalogSheet(item: CatalogItem, catalog: BotCatalog, added: Boolean, busy: Boolean, add: () -> Unit) {
    val title = when (item) { is CatalogItem.Bot -> item.profile.name; is CatalogItem.Team -> item.team.name }
    val summary = when (item) { is CatalogItem.Bot -> item.profile.summary; is CatalogItem.Team -> item.team.summary }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(title) {
            when {
                added -> Box(Modifier.size(32.dp), contentAlignment = Alignment.Center) { Symbol("checkmark", size = 16.dp, tint = Palette.secondary, contentDescription = "Added") }
                busy -> Box(Modifier.size(36.dp), contentAlignment = Alignment.Center) { Spinner() }
                else -> IconButton("Add", "plus", action = add)
            }
        }
        CardForm(Modifier.weight(1f)) {
            CardSection { row { Text(summary, style = Type.subheadline, color = Palette.secondary) } }
            when (item) {
                is CatalogItem.Bot -> CardSection("Instructions", "You can change them in the bot's settings after adding it.") {
                    row { SelectionContainer { Text(item.profile.instructions, style = Type.body, color = Palette.text) } }
                }
                is CatalogItem.Team -> CardSection("Team", "The coordinator leads the group chat and hands each request to the right teammate. Each bot can be changed in its settings.") {
                    catalog.profiles(item.team).forEachIndexed { index, p ->
                        row {
                            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                                ProfileFace(p, 32.dp)
                                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                    Text(if (index == 0) tr("{name} · Coordinator").replace("{name}", p.name) else p.name, style = Type.body.semibold, color = Palette.text)
                                    Text(p.summary, style = Type.caption, color = Palette.secondary)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
