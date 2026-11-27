import CodyncKit
import SwiftUI

/// The bot's routines, model and daily use, read from the host's `botInsights` method.
struct BotInsightsSection: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var insights: BotInsights?
    @State private var failed = false

    var body: some View {
        CardSection(L10n.tr("Insights")) {
            if failed {
                Text(L10n.tr("Could not load insights.")).font(.footnote).foregroundStyle(Palette.secondary)
            } else if let insights {
                VStack(alignment: .leading, spacing: 10) {
                    if !insights.description.isEmpty {
                        Text(insights.description).font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    if let provider = insights.provider {
                        Label {
                            Text(provider).font(.subheadline).foregroundStyle(Palette.text)
                        } icon: {
                            Image(systemName: "cpu").foregroundStyle(Palette.secondary)
                        }
                    }
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Tokens")): \(insights.usage.usedText)")
                        Text("\(L10n.tr("Skills")): \(insights.skills.total)")
                    }
                    .font(.footnote)
                    .foregroundStyle(Palette.secondary)
                    let router = insights.router
                    Text(router.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Selected")): \(router.selected.joined(separator: ", "))").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Strategy")): \(router.strategy)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let cmdpolicy = insights.cmdpolicy
                    Text(cmdpolicy.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Verdict")): \(cmdpolicy.verdict)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Read only")): \(cmdpolicy.readOnly)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let missions = insights.missions
                    Text(missions.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Cells")): \(missions.cells.joined(separator: " · "))").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Span")): \(missions.span)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let noai = insights.noai
                    Text(noai.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("State")): \(noai.state)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Brain")): \(noai.brain)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let devices = insights.devices
                    Text(devices.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("All")): \(devices.all)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Look")): \(devices.look)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let vault = insights.vault
                    Text(vault.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Title")): \(vault.title)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Policy")): \(vault.policy)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let toolreplay = insights.toolreplay
                    Text(toolreplay.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Kept")): \(toolreplay.kept)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Outcome")): \(toolreplay.outcome)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let secretcleanup = insights.secretcleanup
                    Text(secretcleanup.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Kind")): \(secretcleanup.kind)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Retry due")): \(secretcleanup.retryDue)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let avatar = insights.avatar
                    Text(avatar.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Hex")): \(avatar.hex)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Contrast")): \(avatar.contrast)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let providers = insights.providers
                    Text(providers.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Keys")): \(providers.keys)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Guess")): \(providers.guess)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let plans = insights.plans
                    Text(plans.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Choice")): \(plans.choice)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Key plan")): \(plans.keyPlan)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let accounts = insights.accounts
                    Text(accounts.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("State")): \(accounts.state)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Action")): \(accounts.action)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let engine = insights.engine
                    Text(engine.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Engine")): \(engine.engine)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("State")): \(engine.state)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let brainpicker = insights.brainpicker
                    Text(brainpicker.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Groups")): \(brainpicker.groups.joined(separator: ", "))").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Free")): \(brainpicker.free)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let approvalrules = insights.approvalrules
                    Text(approvalrules.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Tool")): \(approvalrules.tool)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Rules")): \(approvalrules.rules)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let iteration = insights.iteration
                    Text(iteration.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Ghosts")): \(iteration.ghosts)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Restart")): \(iteration.restartLabel)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let rrule = insights.rrule
                    Text(rrule.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Cron")): \(rrule.cron ?? "—")").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Timezone")): \(rrule.timezone)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let autofill = insights.autofill
                    Text(autofill.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("OTP")): \(autofill.otp)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Best")): \(autofill.best)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let verify = insights.verify
                    Text(verify.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Evidence")): \(verify.evidence)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Action")): \(verify.nudgeLabel)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let contextpin = insights.contextpin
                    Text(contextpin.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Marker")): \(contextpin.marker)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Ratio")): \(contextpin.ratio)%").font(.footnote).foregroundStyle(Palette.tertiary)
                    let mdtables = insights.mdtables
                    Text(mdtables.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Rows")): \(mdtables.rows)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Layout")): \(mdtables.layout)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let contextselect = insights.contextselect
                    Text(contextselect.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Available")): \(contextselect.available)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Kept")): \(contextselect.kept)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let picker = insights.picker
                    Text(picker.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(picker.bar).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Used")): \(picker.used)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let palette = insights.palette
                    Text(palette.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Command")): \(palette.command)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Results")): \(palette.results)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let shortcuts = insights.shortcuts
                    Text(shortcuts.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Modifier")): \(shortcuts.modifier)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Quote")): \(shortcuts.quote)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let evalextra = insights.evalextra
                    Text(evalextra.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Median")): \(evalextra.median)").font(.footnote).foregroundStyle(Palette.secondary)
                    let connextra = insights.connextra
                    Text(connextra.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Channels")): \(connextra.channels.joined(separator: ", "))").font(.footnote).foregroundStyle(Palette.secondary)
                    let searchextra = insights.searchextra
                    Text(searchextra.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Query")): \(searchextra.query)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Matched")): \(searchextra.matched)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let memoryctx = insights.memoryctx
                    Text(memoryctx.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(memoryctx.preview).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Hidden")): \(memoryctx.hidden)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let apierror = insights.apierror
                    Text(apierror.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(apierror.preview).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Class")): \(apierror.classLabel)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let history = insights.history
                    Text(history.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(history.preview).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Withheld")): \(history.withheld)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let review = insights.review
                    Text(review.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Mode")): \(review.mode)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Queued")): \(review.queued)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let blueprint = insights.blueprint
                    Text(blueprint.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Cron")): \(blueprint.cron)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Schedule")): \(blueprint.schedule)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let stream = insights.stream
                    Text(stream.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Held")): \(stream.held)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Chunks")): \(stream.chunks)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let i18n = insights.i18n
                    Text(i18n.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Intl")): \(i18n.intl)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Plural")): \(i18n.plural)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let quota = insights.quota
                    Text(quota.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Status")): \(quota.status)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Remaining")): \(quota.remaining)%").font(.footnote).foregroundStyle(Palette.tertiary)
                    let appearance = insights.appearance
                    Text(appearance.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Appearance")): \(appearance.appearance)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Body size")): \(appearance.bodySize)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let connstate = insights.connstate
                    Text(connstate.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("State")): \(connstate.state)").font(.footnote).foregroundStyle(Palette.secondary)
                    let evalResult = insights.eval
                    Text(evalResult.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Verdict")): \(evalResult.verdict)").font(.footnote).foregroundStyle(Palette.secondary)
                    let search = insights.search
                    Text(search.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Hits")): \(search.hits)").font(.footnote).foregroundStyle(Palette.secondary)
                    let mailtext = insights.mailtext
                    Text(mailtext.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Address")): \(mailtext.address)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Rows")): \(mailtext.rows)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let notify = insights.notify
                    Text(notify.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("State")): \(notify.state)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Failed")): \(notify.failed)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let markdown = insights.markdown
                    Text(markdown.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(markdown.preview).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Mentions")): \(markdown.mentions)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let localfmt = insights.localfmt
                    Text(localfmt.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Weekday")): \(localfmt.weekday)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Tokens")): \(localfmt.tokens)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let textfmt = insights.textfmt
                    Text(textfmt.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Handle")): \(textfmt.handle)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Hidden marks")): \(textfmt.hidden)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let agenteschat = insights.agenteschat
                    Text(agenteschat.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Window")): \(agenteschat.window)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Mode")): \(agenteschat.auto)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let openbot = insights.openbot
                    Text(openbot.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Effort")): \(openbot.effort)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Route")): \(openbot.route)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let opendots = insights.opendots
                    Text(opendots.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Container")): \(opendots.container)").font(.footnote).foregroundStyle(Palette.secondary)
                    let context = insights.context
                    Text(context.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Page states")): \(context.page)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Screenshots")): \(context.screenshots)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let liveness = insights.liveness
                    Text(liveness.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Delivery")): \(liveness.outcome)").font(.footnote).foregroundStyle(Palette.secondary)
                    let dictation = insights.dictation
                    Text(dictation.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Engine")): \(dictation.engine)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Model")): \(dictation.model)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let access = insights.access
                    Text(access.mode).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("Routing")): \(access.routing)").font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(access.collections, id: \.name) { item in
                        Text(item.name).font(.footnote).foregroundStyle(item.valid ? Palette.text : Palette.danger)
                    }
                    let blocks = insights.blocks
                    Text(blocks.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(blocks.origin).font(.footnote).foregroundStyle(Palette.tertiary)
                    ForEach(blocks.blocks, id: \.kind) { item in
                        Text("\(item.says ? "â—" : "â—‹") \(item.text)").font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    let directory = insights.directory
                    Text(directory.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(directory.bar).font(.footnote).foregroundStyle(Palette.accent)
                    ForEach(directory.integrations, id: \.id) { item in
                        Text("\(item.name) Â· \(item.header)").font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    let prompts = insights.prompts
                    Text(prompts.model).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(prompts.price).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(prompts.context) Â· \(prompts.zone) Â· \(prompts.length)").font(.footnote).foregroundStyle(Palette.tertiary)
                    ForEach(prompts.lines, id: \.self) { line in
                        Text(line).font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    let fresh = insights.fresh
                    Text(fresh.agent).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    ForEach(fresh.summary, id: \.key) { row in
                        Text("\(row.key): \(row.value)").font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    Text("\(fresh.titles.exec) Â· \(fresh.titles.write)").font(.footnote).foregroundStyle(Palette.tertiary)
                    let agent = insights.agent
                    Text(agent.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(agent.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(agent.lines, id: \.id) { item in
                        Text("\(item.set ? "â—" : "â—‹") \(item.label): \(item.detail)")
                            .font(.footnote).foregroundStyle(item.set ? Palette.text : Palette.secondary)
                    }
                    let conversation = insights.conversation
                    Text(conversation.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(conversation.line).font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(conversation.kinds, id: \.kind) { kind in
                        HStack(spacing: 6) {
                            Text(kind.label).font(.footnote).foregroundStyle(Palette.secondary)
                            Text(kind.bar).font(.footnote).foregroundStyle(Palette.accent)
                            Text("\(kind.count)").font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    let failures = insights.failures
                    if failures.total != "0" {
                        Text(failures.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                        ForEach(failures.categories, id: \.kind) { cat in
                            HStack(spacing: 6) {
                                Text(cat.kind).font(.footnote).foregroundStyle(Palette.secondary)
                                Text(cat.bar).font(.footnote).foregroundStyle(Palette.accent)
                                Text("\(cat.count)").font(.footnote).foregroundStyle(Palette.tertiary)
                            }
                        }
                        ForEach(failures.items, id: \.reason) { item in
                            VStack(alignment: .leading, spacing: 1) {
                                Text(item.name).font(.footnote).foregroundStyle(Palette.text)
                                Text("\(item.reason) Â· \(item.age)").font(.footnote).foregroundStyle(Palette.tertiary)
                            }
                        }
                    }
                    let approvals = insights.approvals
                    Text(approvals.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(approvals.bar).font(.footnote).foregroundStyle(Palette.accent)
                    ForEach(approvals.items, id: \.id) { item in
                        VStack(alignment: .leading, spacing: 1) {
                            Text(item.title).font(.footnote).foregroundStyle(Palette.text)
                            Text("\(item.detail) Â· \(item.age)").font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    let zones = insights.timezone
                    Text(zones.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text("\(L10n.tr("This computer")): \(zones.local)").font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(zones.all, id: \.name) { zone in
                        HStack(spacing: 6) {
                            Text(zone.label).font(.footnote).foregroundStyle(Palette.secondary)
                            Text(zone.bar).font(.footnote).foregroundStyle(Palette.accent)
                            Text("\(zone.count)").font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    let digest = insights.digest
                    Text(digest.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(digest.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    Text(digest.oneLine).font(.footnote).foregroundStyle(Palette.tertiary)
                    ForEach(digest.stats, id: \.id) { item in
                        HStack(spacing: 6) {
                            Text(item.label).font(.footnote).foregroundStyle(Palette.secondary)
                            Text(item.value).font(.footnote).foregroundStyle(Palette.accent)
                        }
                    }
                    let advice = insights.advice
                    Text(advice.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(advice.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(advice.items, id: \.id) { item in
                        VStack(alignment: .leading, spacing: 1) {
                            Text("\(item.priorityLabel): \(item.title)").font(.footnote).foregroundStyle(Palette.secondary)
                            Text(item.detail).font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    let session = insights.session
                    Text(session.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(session.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    Text(session.bar).font(.footnote).foregroundStyle(Palette.accent)
                    ForEach(session.lines, id: \.id) { item in
                        Text("\(item.ok ? "â—" : "â—‹") \(item.label): \(item.detail)")
                            .font(.footnote).foregroundStyle(item.ok ? Palette.text : Palette.secondary)
                    }
                    let workspace = insights.workspace
                    Text(workspace.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(workspace.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(workspace.lines, id: \.id) { item in
                        Text("\(item.set ? "â—" : "â—‹") \(item.label): \(item.detail)")
                            .font(.footnote).foregroundStyle(item.set ? Palette.text : Palette.secondary)
                    }
                    let memory = insights.memory
                    Text(memory.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(memory.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Shared notes")): \(memory.shared)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text(memory.sharedDetail).font(.footnote).foregroundStyle(Palette.tertiary)
                    ForEach(memory.lines, id: \.id) { item in
                        Text("\(item.on ? "â—" : "â—‹") \(item.title): \(item.detail)")
                            .font(.footnote).foregroundStyle(item.on ? Palette.text : Palette.secondary)
                    }
                    let tuning = insights.tuning
                    Text(tuning.effort).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(tuning.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(tuning.options, id: \.key) { opt in
                        Text("\(opt.title): \(opt.value)").font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    let accounts = insights.connectors
                    Text(accounts.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    ForEach(accounts.categories, id: \.kind) { cat in
                        HStack(spacing: 6) {
                            Text(cat.kind).font(.footnote).foregroundStyle(Palette.secondary)
                            Text(cat.bar).font(.footnote).foregroundStyle(Palette.accent)
                            Text("\(cat.count)").font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    ForEach(accounts.connectors, id: \.name) { item in
                        VStack(alignment: .leading, spacing: 1) {
                            Text(item.name).font(.footnote).foregroundStyle(Palette.text)
                            Text(item.detail).font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    let identity = insights.identity
                    Text(identity.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Roster")): \(identity.band)")
                        Text("\(L10n.tr("Section")): \(identity.section)")
                        Text("\(L10n.tr("Rank")): \(identity.rank)")
                    }
                    .font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(identity.color) Â· \(identity.shape) Â· \(identity.look)")
                        .font(.footnote).foregroundStyle(Palette.tertiary)
                    ForEach(identity.traits, id: \.id) { trait in
                        Text("\(trait.on ? "â—" : "â—‹") \(trait.label): \(trait.detail)")
                            .font(.footnote).foregroundStyle(trait.on ? Palette.text : Palette.secondary)
                    }
                    let resilience = insights.resilience
                    Text(resilience.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(resilience.bar).font(.footnote).foregroundStyle(Palette.accent)
                    Text(resilience.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    Text(resilience.switches.map { "\($0.on ? "â—" : "â—‹") \($0.label)" }.joined(separator: "  "))
                        .font(.footnote).foregroundStyle(Palette.tertiary)
                    ForEach(resilience.fallbacks, id: \.backend) { fb in
                        Text(fb.label).font(.footnote).foregroundStyle(Palette.text)
                    }
                    let trust = insights.trust
                    Text(trust.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(trust.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    Text("\(L10n.tr("Approval")): \(trust.modeLabel)").font(.footnote).foregroundStyle(Palette.secondary)
                    Text(trust.modeDetail).font(.footnote).foregroundStyle(Palette.tertiary)
                    ForEach(trust.rules, id: \.pattern) { rule in
                        VStack(alignment: .leading, spacing: 1) {
                            Text("\(rule.decision): \(rule.pattern)").font(.footnote).foregroundStyle(Palette.text)
                            if !rule.kind.isEmpty { Text(rule.kind).font(.footnote).foregroundStyle(Palette.tertiary) }
                        }
                    }
                    let library = insights.library
                    Text(library.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    ForEach(library.kinds, id: \.kind) { kind in
                        HStack(spacing: 6) {
                            Text(kind.kind).font(.footnote).foregroundStyle(Palette.secondary)
                            Text(kind.bar).font(.footnote).foregroundStyle(Palette.accent)
                            Text("\(kind.count)").font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    ForEach(library.skills.prefix(8), id: \.name) { skill in
                        VStack(alignment: .leading, spacing: 1) {
                            Text(skill.name).font(.footnote).foregroundStyle(Palette.text)
                            Text(skill.detail).font(.footnote).foregroundStyle(Palette.tertiary)
                        }
                    }
                    let schedule = insights.routineSummary
                    Text(schedule.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(schedule.bar).font(.footnote).foregroundStyle(Palette.accent)
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Active")): \(schedule.active)")
                        Text("\(L10n.tr("Paused")): \(schedule.paused)")
                        Text(schedule.mix)
                    }
                    .font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(schedule.soonest, id: \.id) { item in
                        HStack(spacing: 6) {
                            Text(item.enabled ? "â—" : "â—‹").foregroundStyle(Palette.secondary)
                            Text("\(item.name) Â· \(item.trigger)").font(.footnote).foregroundStyle(Palette.text)
                            if !item.nextIn.isEmpty {
                                Text(item.nextIn).font(.footnote).foregroundStyle(Palette.tertiary)
                            }
                        }
                    }
                    let week = insights.week
                    Text(week.verdict).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(week.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Steps")): \(week.totalLabel)")
                        Text("\(L10n.tr("Trend")): \(week.trend.label)")
                    }
                    .font(.footnote).foregroundStyle(Palette.secondary)
                    if let peak = week.peak {
                        Text("\(L10n.tr("Busiest")): \(peak.label) Â· \(peak.count)")
                            .font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    ForEach(week.top, id: \.label) { day in
                        HStack(spacing: 6) {
                            Text(day.label).font(.footnote).foregroundStyle(Palette.tertiary)
                            Text(day.bar).font(.footnote).foregroundStyle(Palette.accent)
                            Text("\(day.count)").font(.footnote).foregroundStyle(Palette.secondary)
                        }
                    }
                    let profile = insights.profile
                    Text(profile.bar).font(.footnote).foregroundStyle(Palette.accent)
                    Text(profile.headline).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    if !profile.nextUp.isEmpty {
                        Text("\(L10n.tr("Next up")): \(profile.nextUp.map(\.title).joined(separator: " Â· "))")
                            .font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    ForEach(profile.caps) { cap in
                        Text("\(cap.enabled ? "â—" : "â—‹") \(cap.title): \(cap.detail)")
                            .font(.footnote)
                            .foregroundStyle(cap.enabled ? Palette.text : Palette.secondary)
                    }
                    let health = insights.health
                    Text(health.label).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                    Text(health.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    ForEach(health.checks) { item in
                        Text("\(item.title): \(item.detail)")
                            .font(.footnote)
                            .foregroundStyle(item.severity == "error" ? Palette.danger : Palette.secondary)
                    }
                    let activity = insights.activity
                    Text(activity.headline).font(.footnote).foregroundStyle(Palette.secondary)
                    HStack(spacing: 12) {
                        Text("\(L10n.tr("Steps")): \(activity.steps)")
                        if activity.errors > 0 {
                            Text("\(activity.errors) \(L10n.tr("failed"))").foregroundStyle(Palette.danger)
                        }
                    }
                    .font(.footnote)
                    .foregroundStyle(Palette.secondary)
                    if !activity.groups.isEmpty {
                        Text(activity.groups.prefix(6).map { "\($0.count) \($0.label)" }.joined(separator: " Â· "))
                            .font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    if !activity.days.isEmpty {
                        ForEach(activity.days.suffix(5)) { day in
                            HStack(spacing: 6) {
                                Text(day.label).font(.footnote).foregroundStyle(Palette.tertiary)
                                Text(String(repeating: "â–®", count: max(1, day.width)))
                                    .font(.footnote).foregroundStyle(Palette.accent)
                                Text("\(day.count)").font(.footnote).foregroundStyle(Palette.secondary)
                            }
                        }
                    }
                    ForEach(activity.items.prefix(6), id: \.text) { step in
                        VStack(alignment: .leading, spacing: 1) {
                            Text(step.text)
                                .font(.footnote)
                                .foregroundStyle(step.status == "error" ? Palette.danger : Palette.text)
                            if !step.detail.isEmpty {
                                Text(step.detail).font(.footnote).foregroundStyle(Palette.tertiary)
                            }
                        }
                    }
                    if !insights.recentWork.isEmpty {
                        Text(L10n.tr("Recent work")).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                        ForEach(insights.recentWork, id: \.self) { line in
                            Text(line).font(.footnote).foregroundStyle(Palette.secondary)
                        }
                    }
                    if insights.routines.isEmpty {
                        Text(L10n.tr("No routines yet.")).font(.footnote).foregroundStyle(Palette.tertiary)
                    } else {
                        ForEach(insights.routines) { routine in
                            VStack(alignment: .leading, spacing: 2) {
                                HStack(spacing: 6) {
                                    Text(routine.name).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                                    if !routine.enabled {
                                        Text(L10n.tr("Paused")).font(.footnote).foregroundStyle(Palette.secondary)
                                    }
                                }
                                if !routine.schedules.isEmpty {
                                    Text(routine.schedules.joined(separator: " Â· "))
                                        .font(.footnote).foregroundStyle(Palette.secondary)
                                }
                                if let next = routine.nextIn {
                                    Text(L10n.tr("Next in {time}").replacingOccurrences(of: "{time}", with: next))
                                        .font(.footnote).foregroundStyle(Palette.tertiary)
                                }
                            }
                        }
                    }
                }
            } else {
                Text(L10n.tr("Loadingâ€¦")).font(.footnote).foregroundStyle(Palette.secondary)
            }
        }
        .task(id: botId) { await load() }
    }

    private func load() async {
        insights = nil
        failed = false
        guard let client = model.client else { return }
        do {
            insights = try await client.call("botInsights", ["botId": botId])
        } catch {
            failed = true
        }
    }
}
