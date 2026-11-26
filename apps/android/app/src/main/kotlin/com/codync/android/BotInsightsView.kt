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
            val mailtext = insights.mailtext
            CardSection("Mail & data") {
                row { Text(mailtext.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Address", mailtext.address) }
                row { ValueRow("Rows", mailtext.rows.toString()) }
                row { ValueRow("Keys", mailtext.keys.toString()) }
            }
            val notify = insights.notify
            CardSection("Notifications") {
                row { Text(notify.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("State", notify.state) }
                row { ValueRow("Diagnostic", notify.diagnostic.toString()) }
                row { ValueRow("Failed", notify.failed.toString()) }
            }
            val markdown = insights.markdown
            CardSection("Markdown") {
                row { Text(markdown.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(markdown.preview, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Blocks", markdown.blocks.toString()) }
                row { ValueRow("Mentions", markdown.mentions.toString()) }
            }
            val localfmt = insights.localfmt
            CardSection("Local format") {
                row { Text(localfmt.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Clock", localfmt.clock) }
                row { ValueRow("Weekday", localfmt.weekday) }
                row { ValueRow("Tokens", localfmt.tokens) }
                row { ValueRow("Budget", localfmt.budget) }
            }
            val textfmt = insights.textfmt
            CardSection("Text hygiene") {
                row { Text(textfmt.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Handle", textfmt.handle) }
                row { ValueRow("Invalid", textfmt.invalidEmails.toString()) }
                row { ValueRow("Hidden", textfmt.hidden.toString()) }
            }
            val agenteschat = insights.agenteschat
            CardSection("Context budget") {
                row { Text(agenteschat.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Window", agenteschat.window) }
                row { ValueRow("Mode", agenteschat.auto) }
            }
            val openbot = insights.openbot
            CardSection("Run model") {
                row { Text(openbot.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Effort", openbot.effort) }
                row { ValueRow("Route", openbot.route) }
            }
            val opendots = insights.opendots
            CardSection("Learning") {
                row { Text(opendots.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Container", opendots.container) }
                row { ValueRow("Internal", opendots.internal.toString()) }
            }
            val context = insights.context
            CardSection("Context window") {
                row { Text(context.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Page states", context.page) }
                row { ValueRow("Screenshots", context.screenshots) }
                row { Text(context.pageNote, style = Type.caption, color = Palette.tertiary) }
            }
            val liveness = insights.liveness
            CardSection("Liveness") {
                row { Text(liveness.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("State", liveness.state) }
                row { ValueRow("Delivery", liveness.outcome) }
            }
            val dictation = insights.dictation
            CardSection("Voice dictation") {
                row { Text(dictation.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Engine", dictation.engine) }
                row { ValueRow("Model", dictation.model) }
                row { ValueRow("Language", dictation.language) }
            }
            val access = insights.access
            CardSection("Public access") {
                row { Text(access.mode, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Routing", access.routing) }
                for (item in access.collections) row {
                    Text(item.name, style = Type.caption, color = if (item.valid) Palette.text else Palette.danger)
                }
            }
            val blocks = insights.blocks
            CardSection("Last message") {
                row { Text(blocks.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(blocks.origin, style = Type.caption, color = Palette.tertiary) }
                for (item in blocks.blocks) row {
                    Text("${if (item.says) "●" else "○"} ${item.text}", style = Type.caption, color = Palette.secondary)
                }
            }
            val directory = insights.directory
            CardSection("Directory") {
                row { Text(directory.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(directory.bar, style = Type.caption, color = Palette.accent) }
                row { ValueRow("Integrations", directory.integrations.size.toString()) }
                for (item in directory.integrations) row {
                    Text("${item.name} · ${item.header}", style = Type.caption, color = Palette.secondary)
                }
            }
            val prompts = insights.prompts
            CardSection("Prompt style") {
                row { Text(prompts.model, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(prompts.price, style = Type.caption, color = Palette.secondary) }
                row { Text("${prompts.context} · ${prompts.zone} · ${prompts.length}", style = Type.caption, color = Palette.tertiary) }
                for (line in prompts.lines) row {
                    Text(line, style = Type.caption, color = Palette.secondary)
                }
            }
            val fresh = insights.fresh
            CardSection("Latest from bdots") {
                row { Text(fresh.agent, style = Type.subheadline.semibold, color = Palette.text) }
                for (row in fresh.summary) row {
                    Text("${row.key}: ${row.value}", style = Type.caption, color = Palette.secondary)
                }
                row { Text("${fresh.titles.exec} · ${fresh.titles.write}", style = Type.caption, color = Palette.tertiary) }
            }
            val agent = insights.agent
            CardSection("Agent profile") {
                row { Text(agent.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(agent.headline, style = Type.caption, color = Palette.secondary) }
                for (item in agent.lines) row {
                    Text("${if (item.set) "●" else "○"} ${item.label}: ${item.detail}",
                        style = Type.caption, color = if (item.set) Palette.text else Palette.secondary)
                }
            }
            val conversation = insights.conversation
            CardSection("Conversation") {
                row { Text(conversation.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(conversation.line, style = Type.caption, color = Palette.secondary) }
                for (kind in conversation.kinds) row {
                    Text("${kind.label}  ${kind.bar}  ${kind.count}", style = Type.caption, color = Palette.tertiary)
                }
            }
            val failures = insights.failures
            if (failures.total != "0") CardSection("Recent failures") {
                row { Text(failures.headline, style = Type.subheadline.semibold, color = Palette.text) }
                for (cat in failures.categories) row {
                    Text("${cat.kind}  ${cat.bar}  ${cat.count}", style = Type.caption, color = Palette.tertiary)
                }
                for (item in failures.items) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(item.name, style = Type.caption, color = Palette.text)
                        Text("${item.reason} · ${item.age}", style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            val approvals = insights.approvals
            CardSection("Approvals") {
                row { Text(approvals.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(approvals.bar, style = Type.caption, color = Palette.accent) }
                for (item in approvals.items) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(item.title, style = Type.caption, color = Palette.text)
                        Text("${item.detail} · ${item.age}", style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            val zones = insights.timezone
            CardSection("Time zones") {
                row { Text(zones.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text("This computer: ${zones.local}", style = Type.caption, color = Palette.secondary) }
                for (zone in zones.all) row {
                    Text("${zone.label}  ${zone.bar}  ${zone.count}", style = Type.caption, color = Palette.tertiary)
                }
            }
            val digest = insights.digest
            CardSection("At a glance") {
                row { Text(digest.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(digest.headline, style = Type.caption, color = Palette.secondary) }
                row { Text(digest.oneLine, style = Type.caption, color = Palette.tertiary) }
                for (item in digest.stats) row {
                    Text("${item.label}: ${item.value}", style = Type.caption, color = Palette.secondary)
                }
            }
            val advice = insights.advice
            CardSection("Suggestions") {
                row { Text(advice.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(advice.headline, style = Type.caption, color = Palette.secondary) }
                for (item in advice.items) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text("${item.priorityLabel}: ${item.title}", style = Type.caption, color = Palette.secondary)
                        Text(item.detail, style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            val session = insights.session
            CardSection("Session") {
                row { Text(session.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(session.headline, style = Type.caption, color = Palette.secondary) }
                row { Text(session.bar, style = Type.caption, color = Palette.accent) }
                row { ValueRow("Unread", session.unread) }
                for (item in session.lines) row {
                    Text("${if (item.ok) "●" else "○"} ${item.label}: ${item.detail}",
                        style = Type.caption, color = if (item.ok) Palette.text else Palette.secondary)
                }
            }
            val workspace = insights.workspace
            CardSection("Workspace") {
                row { Text(workspace.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(workspace.headline, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Folder", workspace.folder) }
                for (item in workspace.lines) row {
                    Text("${if (item.set) "●" else "○"} ${item.label}: ${item.detail}",
                        style = Type.caption, color = if (item.set) Palette.text else Palette.secondary)
                }
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
