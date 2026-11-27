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
            row { Text("LoadingÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦", style = Type.body, color = Palette.secondary) }
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
            val cmdpolicy = insights.cmdpolicy
            CardSection("Command policy") {
                row { Text(cmdpolicy.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Verdict", cmdpolicy.verdict) }
                row { ValueRow("Denied", cmdpolicy.denied.toString()) }
                row { ValueRow("Read only", cmdpolicy.readOnly.toString()) }
            }
            val missions = insights.missions
            CardSection("Missions") {
                row { Text(missions.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Cells", missions.cells.joinToString(" · ")) }
                row { ValueRow("Span", missions.span) }
                row { ValueRow("Open", missions.open.toString()) }
            }
            val noai = insights.noai
            CardSection("No AI") {
                row { Text(noai.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("State", noai.state) }
                row { ValueRow("Brain", noai.brain) }
                row { ValueRow("Choices", noai.choices.toString()) }
            }
            val devices = insights.devices
            CardSection("Devices") {
                row { Text(devices.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("All", devices.all.toString()) }
                row { ValueRow("Look", devices.look) }
                row { ValueRow("Link", devices.link) }
            }
            val vault = insights.vault
            CardSection("Vault") {
                row { Text(vault.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Title", vault.title) }
                row { ValueRow("Policy", vault.policy) }
                row { ValueRow("Last 4", vault.card.last4) }
            }
            val toolreplay = insights.toolreplay
            CardSection("Tool replay") {
                row { Text(toolreplay.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Kept", toolreplay.kept.toString()) }
                row { ValueRow("Replay", toolreplay.replay.toString()) }
                row { ValueRow("Outcome", toolreplay.outcome) }
            }
            val secretcleanup = insights.secretcleanup
            CardSection("Secret cleanup") {
                row { Text(secretcleanup.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Kind", secretcleanup.kind) }
                row { ValueRow("Retry due", secretcleanup.retryDue.toString()) }
                row { ValueRow("Backup due", secretcleanup.backupDue.toString()) }
            }
            val avatar = insights.avatar
            CardSection("Avatar") {
                row { Text(avatar.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Hex", avatar.hex) }
                row { ValueRow("Eye", avatar.eye) }
                row { ValueRow("Contrast", avatar.contrast) }
            }
            val providers = insights.providers
            CardSection("Providers") {
                row { Text(providers.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Keys", providers.keys.toString()) }
                row { ValueRow("Usable models", providers.usable.toString()) }
                row { ValueRow("Guess", providers.guess) }
            }
            val plans = insights.plans
            CardSection("Coding plans") {
                row { Text(plans.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Provider plan", plans.providerPlan ?: "—") }
                row { ValueRow("Choice", plans.choice) }
                row { ValueRow("Key plan", plans.keyPlan.toString()) }
            }
            val accounts = insights.accounts
            CardSection("Accounts") {
                row { Text(accounts.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("State", accounts.state) }
                row { ValueRow("Action", accounts.action) }
                row { ValueRow("Title", accounts.title) }
            }
            val engine = insights.engine
            CardSection("Engine") {
                row { Text(engine.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Engine", engine.engine) }
                row { ValueRow("State", engine.state) }
                row { ValueRow("Line", engine.line) }
            }
            val brainpicker = insights.brainpicker
            CardSection("Brain picker") {
                row { Text(brainpicker.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Groups", brainpicker.groups.joinToString(", ")) }
                row { ValueRow("Free", brainpicker.free.toString()) }
                row { ValueRow("Keywords", brainpicker.keywords) }
            }
            val approvalrules = insights.approvalrules
            CardSection("Approval rules") {
                row { Text(approvalrules.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Tool", approvalrules.tool) }
                row { ValueRow("SQL read", approvalrules.sqlRead.toString()) }
                row { ValueRow("Rules", approvalrules.rules.toString()) }
            }
            val iteration = insights.iteration
            CardSection("Iteration") {
                row { Text(iteration.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Ghosts", iteration.ghosts.toString()) }
                row { ValueRow("Tools", iteration.tools.toString()) }
                row { ValueRow("Restart", iteration.restartLabel) }
            }
            val rrule = insights.rrule
            CardSection("Recurrence") {
                row { Text(rrule.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Cron", rrule.cron ?: "—") }
                row { ValueRow("Weekdays", rrule.weekdays.joinToString(", ")) }
                row { ValueRow("Timezone", rrule.timezone) }
            }
            val autofill = insights.autofill
            CardSection("Autofill") {
                row { Text(autofill.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("OTP", autofill.otp.toString()) }
                row { ValueRow("Checkout", autofill.checkout.toString()) }
                row { ValueRow("Best", autofill.best) }
            }
            val verify = insights.verify
            CardSection("Verification") {
                row { Text(verify.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Evidence", verify.evidence) }
                row { ValueRow("Paths", verify.paths.toString()) }
                row { ValueRow("Action", verify.nudgeLabel) }
            }
            val contextpin = insights.contextpin
            CardSection("Window pin") {
                row { Text(contextpin.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Marker", contextpin.marker) }
                row { ValueRow("Pinned", contextpin.pinned.toString()) }
                row { ValueRow("Ratio", "${contextpin.ratio}%") }
            }
            val mdtables = insights.mdtables
            CardSection("Markdown tables") {
                row { Text(mdtables.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Rows", mdtables.rows.toString()) }
                row { ValueRow("Layout", mdtables.layout) }
                row { ValueRow("Aligned", mdtables.aligned.toString()) }
            }
            val contextselect = insights.contextselect
            CardSection("Context budget") {
                row { Text(contextselect.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Available", contextselect.available.toString()) }
                row { ValueRow("Kept", contextselect.kept.toString()) }
                row { ValueRow("Cache", contextselect.cache) }
            }
            val picker = insights.picker
            CardSection("Usage picker") {
                row { Text(picker.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(picker.bar, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Used", picker.used) }
                row { ValueRow("Resets", picker.resets) }
            }
            val palette = insights.palette
            CardSection("Command palette") {
                row { Text(palette.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Command", palette.command) }
                row { ValueRow("Results", palette.results.toString()) }
            }
            val shortcuts = insights.shortcuts
            CardSection("Shortcuts") {
                row { Text(shortcuts.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Modifier", shortcuts.modifier) }
                row { ValueRow("Quote", shortcuts.quote) }
                row { ValueRow("Command menu", shortcuts.commandMenu ?: "—") }
            }
            val evalextra = insights.evalextra
            CardSection("Eval metrics") {
                row { Text(evalextra.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Median", evalextra.median.toString()) }
                row { ValueRow("Over 1k", evalextra.over1k.toString()) }
            }
            val connextra = insights.connextra
            CardSection("Connections extra") {
                row { Text(connextra.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Channels", connextra.channels.joinToString(", ")) }
                row { ValueRow("Selected", connextra.selected.toString()) }
            }
            val searchextra = insights.searchextra
            CardSection("Ranked search") {
                row { Text(searchextra.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Query", searchextra.query) }
                row { ValueRow("Matched", searchextra.matched.toString()) }
                row { ValueRow("Sort", searchextra.sort) }
            }
            val memoryctx = insights.memoryctx
            CardSection("Memory context") {
                row { Text(memoryctx.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(memoryctx.preview, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Notes", memoryctx.notes.toString()) }
                row { ValueRow("Hidden", memoryctx.hidden.toString()) }
            }
            val apierror = insights.apierror
            CardSection("API errors") {
                row { Text(apierror.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(apierror.preview, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Class", apierror.classLabel) }
                row { ValueRow("Retryable", apierror.retryable.toString()) }
            }
            val history = insights.history
            CardSection("History") {
                row { Text(history.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(history.preview, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Commentary", history.commentary.toString()) }
                row { ValueRow("Withheld", history.withheld.toString()) }
            }
            val review = insights.review
            CardSection("Review queue") {
                row { Text(review.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Mode", review.mode) }
                row { ValueRow("Queued", review.queued.toString()) }
                row { ValueRow("Dispatchable", review.dispatchable.size.toString()) }
            }
            val blueprint = insights.blueprint
            CardSection("Blueprint") {
                row { Text(blueprint.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Cron", blueprint.cron) }
                row { ValueRow("Schedule", blueprint.schedule) }
                row { ValueRow("Command", blueprint.slash) }
            }
            val stream = insights.stream
            CardSection("Stream") {
                row { Text(stream.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Held", stream.held.toString()) }
                row { ValueRow("Consumers", stream.consumers.toString()) }
                row { ValueRow("Chunks", stream.chunks.toString()) }
            }
            val i18n = insights.i18n
            CardSection("Locale") {
                row { Text(i18n.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Intl", i18n.intl) }
                row { ValueRow("Week starts", i18n.weekLabel) }
                row { ValueRow("Plural", i18n.plural) }
            }
            val quota = insights.quota
            CardSection("Quota") {
                row { Text(quota.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Status", quota.status) }
                row { ValueRow("Remaining", "${quota.remaining}%") }
                row { ValueRow("Resets", quota.resetPhrase) }
            }
            val appearance = insights.appearance
            CardSection("Appearance") {
                row { Text(appearance.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Appearance", appearance.appearance) }
                row { ValueRow("Accent", appearance.accent) }
                row { ValueRow("Body size", appearance.bodySize.toString()) }
            }
            val connstate = insights.connstate
            CardSection("Connections") {
                row { Text(connstate.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("State", connstate.state) }
                row { ValueRow("Connected", connstate.connected.toString()) }
            }
            val evalResult = insights.eval
            CardSection("Evaluation") {
                row { Text(evalResult.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Verdict", evalResult.verdict) }
                row { ValueRow("Score", evalResult.score.toString()) }
            }
            val search = insights.search
            CardSection("Search") {
                row { Text(search.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { ValueRow("Query", search.query) }
                row { ValueRow("Hits", search.hits.toString()) }
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
                    Text("${if (item.says) "Ã¢â€”Â" else "Ã¢â€”â€¹"} ${item.text}", style = Type.caption, color = Palette.secondary)
                }
            }
            val directory = insights.directory
            CardSection("Directory") {
                row { Text(directory.headline, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(directory.bar, style = Type.caption, color = Palette.accent) }
                row { ValueRow("Integrations", directory.integrations.size.toString()) }
                for (item in directory.integrations) row {
                    Text("${item.name} Ã‚Â· ${item.header}", style = Type.caption, color = Palette.secondary)
                }
            }
            val prompts = insights.prompts
            CardSection("Prompt style") {
                row { Text(prompts.model, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(prompts.price, style = Type.caption, color = Palette.secondary) }
                row { Text("${prompts.context} Ã‚Â· ${prompts.zone} Ã‚Â· ${prompts.length}", style = Type.caption, color = Palette.tertiary) }
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
                row { Text("${fresh.titles.exec} Ã‚Â· ${fresh.titles.write}", style = Type.caption, color = Palette.tertiary) }
            }
            val agent = insights.agent
            CardSection("Agent profile") {
                row { Text(agent.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(agent.headline, style = Type.caption, color = Palette.secondary) }
                for (item in agent.lines) row {
                    Text("${if (item.set) "Ã¢â€”Â" else "Ã¢â€”â€¹"} ${item.label}: ${item.detail}",
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
                        Text("${item.reason} Ã‚Â· ${item.age}", style = Type.caption, color = Palette.tertiary)
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
                        Text("${item.detail} Ã‚Â· ${item.age}", style = Type.caption, color = Palette.tertiary)
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
                    Text("${if (item.ok) "Ã¢â€”Â" else "Ã¢â€”â€¹"} ${item.label}: ${item.detail}",
                        style = Type.caption, color = if (item.ok) Palette.text else Palette.secondary)
                }
            }
            val workspace = insights.workspace
            CardSection("Workspace") {
                row { Text(workspace.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(workspace.headline, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Folder", workspace.folder) }
                for (item in workspace.lines) row {
                    Text("${if (item.set) "Ã¢â€”Â" else "Ã¢â€”â€¹"} ${item.label}: ${item.detail}",
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
                    Text("${if (item.on) "Ã¢â€”Â" else "Ã¢â€”â€¹"} ${item.title}: ${item.detail}",
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
                row { Text("${identity.color} Ãƒâ€šÃ‚Â· ${identity.shape} Ãƒâ€šÃ‚Â· ${identity.look}", style = Type.caption, color = Palette.tertiary) }
                for (trait in identity.traits) row {
                    Text("${if (trait.on) "ÃƒÂ¢Ã¢â‚¬â€Ã‚Â" else "ÃƒÂ¢Ã¢â‚¬â€Ã¢â‚¬Â¹"} ${trait.label}: ${trait.detail}",
                        style = Type.caption, color = if (trait.on) Palette.text else Palette.secondary)
                }
            }
            val resilience = insights.resilience
            CardSection("Resilience") {
                row { Text(resilience.label, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(resilience.bar, style = Type.caption, color = Palette.accent) }
                row { Text(resilience.headline, style = Type.caption, color = Palette.secondary) }
                if (resilience.switches.isNotEmpty()) row {
                    Text(resilience.switches.joinToString("  ") { "${if (it.on) "ÃƒÂ¢Ã¢â‚¬â€Ã‚Â" else "ÃƒÂ¢Ã¢â‚¬â€Ã¢â‚¬Â¹"} ${it.label}" },
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
                    Text("${if (item.enabled) "ÃƒÂ¢Ã¢â‚¬â€Ã‚Â" else "ÃƒÂ¢Ã¢â‚¬â€Ã¢â‚¬Â¹"} ${item.name} Ãƒâ€šÃ‚Â· ${item.trigger}${if (item.nextIn.isNotBlank()) " Ãƒâ€šÃ‚Â· ${item.nextIn}" else ""}",
                        style = Type.caption, color = Palette.secondary)
                }
            }
            val week = insights.week
            CardSection("This week") {
                row { Text(week.verdict, style = Type.subheadline.semibold, color = Palette.text) }
                row { Text(week.headline, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Steps", week.totalLabel) }
                row { ValueRow("Trend", week.trend.label) }
                week.peak?.let { row { Text("Busiest: ${it.label} Ãƒâ€šÃ‚Â· ${it.count}", style = Type.caption, color = Palette.secondary) } }
                for (day in week.top) row {
                    Text("${day.label}  ${day.bar}  ${day.count}", style = Type.caption, color = Palette.tertiary)
                }
            }
            val profile = insights.profile
            CardSection("Capabilities") {
                row { Text(profile.bar, style = Type.caption, color = Palette.accent) }
                row { Text(profile.headline, style = Type.subheadline.semibold, color = Palette.text) }
                if (profile.nextUp.isNotEmpty()) row {
                    Text("Next up: ${profile.nextUp.joinToString(" Ãƒâ€šÃ‚Â· ") { it.title }}",
                        style = Type.caption, color = Palette.secondary)
                }
                for (cap in profile.caps) row {
                    Text("${if (cap.enabled) "ÃƒÂ¢Ã¢â‚¬â€Ã‚Â" else "ÃƒÂ¢Ã¢â‚¬â€Ã¢â‚¬Â¹"} ${cap.title}: ${cap.detail}",
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
                    Text(activity.groups.take(6).joinToString(" Ãƒâ€šÃ‚Â· ") { "${it.count} ${it.label}" },
                        style = Type.caption, color = Palette.secondary)
                }
                for (day in activity.days.takeLast(5)) row {
                    Text("${day.label}  ${"ÃƒÂ¢Ã¢â‚¬â€œÃ‚Â®".repeat(day.width.coerceAtLeast(1))}  ${day.count}",
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
                            if (routine.schedules.isNotEmpty()) Text(routine.schedules.joinToString(" Ãƒâ€šÃ‚Â· "), style = Type.caption, color = Palette.secondary)
                            routine.nextIn?.let { Text("Next in $it", style = Type.caption, color = Palette.tertiary) }
                        }
                    }
                }
            }
        }
    }
}
