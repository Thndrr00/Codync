package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import com.codync.android.core.AppState
import com.codync.android.core.BotInsights
import com.codync.android.design.*

/** A bot's routines, model and daily use, read from the host's `botInsights` method. */
@Composable internal fun BotInsightsView(botId: String, state: AppState, store: AppStore) {
    var data by remember(botId) { mutableStateOf<BotInsights?>(null) }
    var error by remember(botId) { mutableStateOf<String?>(null) }
    LaunchedEffect(botId, state.generation) {
        data = null
        error = null
        try { data = store.botInsights(botId) } catch (e: Exception) { error = e.message }
    }
    CardForm {
        if (error != null) CardSection("Insights") {
            row { Text("Could not load insights.", style = Type.body, color = Palette.secondary) }
        } else if (data == null) CardSection("Insights") {
            row { Text("Loadingâ€¦", style = Type.body, color = Palette.secondary) }
        } else {
            val insights = data!!
            if (insights.description.isNotBlank()) CardSection("About") {
                row { Text(insights.description, style = Type.caption, color = Palette.secondary) }
            }
            insights.provider?.let { provider ->
                CardSection("Model") { row { ValueRow("Agent", provider) } }
            }
            CardSection("Today") {
                row {
                    ValueRow("Tokens", if (insights.usage.limit != null) "${insights.usage.usedText} / ${insights.usage.limit}" else insights.usage.usedText)
                }
                row { ValueRow("Skills", insights.skills.total.toString()) }
            }
            val memory = insights.memory
            CardSection("Memory") {
                row { Text(memory.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(memory.headline, style = Type.caption, color = Palette.secondary) }
                row { Text("Shared notes: ${memory.shared}", style = Type.caption, color = Palette.secondary) }
                row { Text(memory.sharedDetail, style = Type.caption, color = Palette.tertiary) }
                for (item in memory.lines) row {
                    Text("${if (item.on) "●" else "○"} ${item.title}: ${item.detail}",
                        style = Type.caption, color = if (item.on) Palette.text else Palette.secondary)
                }
            }
            val tuning = insights.tuning
            CardSection("Model tuning") {
                row { Text(tuning.effort, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(tuning.headline, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Options", tuning.count) }
                for (opt in tuning.options) row {
                    Text("${opt.title}: ${opt.value}", style = Type.caption, color = Palette.secondary)
                }
            }
            val accounts = insights.connectors
            CardSection("Connected accounts") {
                row { Text(accounts.headline, style = Type.subheadline.semibold, color = Palette.text) }
                for (cat in accounts.categories) row {
                    Text("${cat.kind}  ${cat.bar}  ${cat.count}", style = Type.caption, color = Palette.tertiary)
                }
                for (item in accounts.connectors) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(item.name, style = Type.caption, color = Palette.text)
                        Text(item.detail, style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            val identity = insights.identity
            CardSection("Identity") {
                row { Text(identity.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text("Roster: ${identity.band}   Section: ${identity.section}   Rank: ${identity.rank}",
                    style = Type.caption, color = Palette.secondary) }
                row { Text("${identity.color} Â· ${identity.shape} Â· ${identity.look}", style = Type.caption, color = Palette.tertiary) }
                for (trait in identity.traits) row {
                    Text("${if (trait.on) "â—" else "â—‹"} ${trait.label}: ${trait.detail}",
                        style = Type.caption, color = if (trait.on) Palette.text else Palette.secondary)
                }
            }
            val resilience = insights.resilience
            CardSection("Resilience") {
                row { Text(resilience.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(resilience.bar, style = Type.caption, color = Palette.accent) }
                row { Text(resilience.headline, style = Type.caption, color = Palette.secondary) }
                if (resilience.switches.isNotEmpty()) row {
                    Text(resilience.switches.joinToString("  ") { "${if (it.on) "â—" else "â—‹"} ${it.label}" },
                        style = Type.caption, color = Palette.tertiary)
                }
                for (fb in resilience.fallbacks) row {
                    Text(fb.label, style = Type.caption, color = Palette.text)
                }
            }
            val trust = insights.trust
            CardSection("Trust") {
                row { Text(trust.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(trust.headline, style = Type.caption, color = Palette.secondary) }
                row { Text("Approval: ${trust.modeLabel}", style = Type.caption, color = Palette.secondary) }
                row { Text(trust.modeDetail, style = Type.caption, color = Palette.tertiary) }
                for (rule in trust.rules) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text("${rule.decision}: ${rule.pattern}", style = Type.caption, color = Palette.text)
                        if (rule.kind.isNotBlank()) Text(rule.kind, style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            val library = insights.library
            CardSection("Skill library") {
                row { Text(library.headline, style = Type.subheadline.semibold, color = Palette.text) }
                for (kind in library.kinds) row {
                    Text("${kind.kind}  ${kind.bar}  ${kind.count}", style = Type.caption, color = Palette.tertiary)
                }
                for (skill in library.skills.take(8)) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(skill.name, style = Type.caption, color = Palette.text)
                        Text(skill.detail, style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            val schedule = insights.routineSummary
            CardSection("Schedule") {
                row { Text(schedule.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(schedule.bar, style = Type.caption, color = Palette.accent) }
                row { Text("Active: ${schedule.active}   Paused: ${schedule.paused}   ${schedule.mix}",
                    style = Type.caption, color = Palette.secondary) }
                for (item in schedule.soonest) row {
                    Text("${if (item.enabled) "â—" else "â—‹"} ${item.name} Â· ${item.trigger}${if (item.nextIn.isNotBlank()) " Â· ${item.nextIn}" else ""}",
                        style = Type.caption, color = Palette.secondary)
                }
            }
            val week = insights.week
            CardSection("This week") {
                row { Text(week.verdict, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(week.headline, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Steps", week.totalLabel) }
                row { ValueRow("Trend", week.trend.label) }
                week.peak?.let { row { Text("Busiest: ${it.label} Â· ${it.count}", style = Type.caption, color = Palette.secondary) } }
                for (day in week.top) row {
                    Text("${day.label}  ${day.bar}  ${day.count}", style = Type.caption, color = Palette.tertiary)
                }
            }
            val profile = insights.profile
            CardSection("Capabilities") {
                row { Text(profile.bar, style = Type.caption, color = Palette.accent) }
                row { Text(profile.headline, style = Type.subheadline.semibold, color = Palette.text) }
                if (profile.nextUp.isNotEmpty()) row {
                    Text("Next up: ${profile.nextUp.joinToString(" Â· ") { it.title }}",
                        style = Type.caption, color = Palette.secondary)
                }
                for (cap in profile.caps) row {
                    Text("${if (cap.enabled) "â—" else "â—‹"} ${cap.title}: ${cap.detail}",
                        style = Type.caption, color = if (cap.enabled) Palette.text else Palette.secondary)
                }
            }
            val health = insights.health
            CardSection("Health") {
                row { Text(health.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(health.headline, style = Type.caption, color = Palette.secondary) }
                for (item in health.checks) row {
                    Text("${item.title}: ${item.detail}", style = Type.caption,
                        color = if (item.severity == "error") Palette.danger else Palette.secondary)
                }
            }
            val activity = insights.activity
            CardSection("Activity") {
                row { Text(activity.headline, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Steps", activity.steps.toString()) }
                if (activity.errors > 0) row {
                    Text("$activity.errors failed", style = Type.caption, color = Palette.danger)
                }
                if (activity.groups.isNotEmpty()) row {
                    Text(activity.groups.take(6).joinToString(" Â· ") { "${it.count} ${it.label}" },
                        style = Type.caption, color = Palette.secondary)
                }
                for (day in activity.days.takeLast(5)) row {
                    Text("${day.label}  ${"â–®".repeat(day.width.coerceAtLeast(1))}  ${day.count}",
                        style = Type.caption, color = Palette.tertiary)
                }
                for (step in activity.items.take(6)) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(step.text, style = Type.caption,
                            color = if (step.status == "error") Palette.danger else Palette.text)
                        if (step.detail.isNotBlank()) Text(step.detail, style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            if (insights.recentWork.isNotEmpty()) CardSection("Recent work") {
                for (line in insights.recentWork) row {
                    Text(line, style = Type.caption, color = Palette.secondary)
                }
            }
            CardSection("Routines") {
                if (insights.routines.isEmpty()) row {
                    Text("No routines yet.", style = Type.body, color = Palette.secondary)
                }
                for (routine in insights.routines) {
                    row {
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(routine.name, style = Type.subheadline.semibold, color = Palette.text)
                            if (!routine.enabled) Text("Paused", style = Type.caption, color = Palette.secondary)
                            if (routine.schedules.isNotEmpty()) Text(routine.schedules.joinToString(" Â· "), style = Type.caption, color = Palette.secondary)
                            routine.nextIn?.let { Text("Next in $it", style = Type.caption, color = Palette.tertiary) }
                        }
                    }
                }
            }
        }
    }
}
