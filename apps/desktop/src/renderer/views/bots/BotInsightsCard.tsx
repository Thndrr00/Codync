import { useEffect, useState } from 'react'
import type { BotInsights } from '@shared/bot-insights'
import { CardSection, ValueRow } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'

/** The bot's routines, model and daily use, read from the host's `botInsights` method. */
export function BotInsightsCard({ botId }: { botId: string }) {
  const store = useStore()
  const tr = useTr()
  const [insights, setInsights] = useState<BotInsights | null>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let live = true
    setInsights(null)
    setError(false)
    void (async () => {
      try {
        const answer = await (await store.ready()).call<BotInsights>('botInsights', { botId })
        if (live) setInsights(answer)
      } catch {
        if (live) setError(true)
      }
    })()
    return () => { live = false }
  }, [botId, store])

  if (error) return <CardSection title={tr('Insights')}><Text size="footnote" tone="secondary">{tr('Could not load insights.')}</Text></CardSection>
  if (!insights) return <CardSection title={tr('Insights')}><Text size="footnote" tone="secondary">{tr('Loading…')}</Text></CardSection>

  const { usage, skills, activity, health, profile, week, routineSummary, library, trust, resilience, identity, connectors, tuning, memory, workspace, session, advice, digest, timezone, approvals, failures, conversation, agent, fresh, prompts, directory, blocks, access, dictation, liveness, context, opendots, openbot, agenteschat, textfmt, localfmt, markdown, notify, mailtext, connstate, eval: evalResult, search, appearance, quota, i18n, stream, blueprint, review, history, apierror, memoryctx, searchextra, evalextra, connextra, shortcuts, palette, picker, contextselect, mdtables, contextpin, verify, autofill, rrule, iteration, approvalrules, brainpicker, engine, accounts, plans, providers, avatar, secretcleanup, toolreplay, vault, devices, noai, missions, cmdpolicy, router, modelprefs, botstore, promptactions, csiterm, plugins, service, draftstash, layout, channelaccess, work, contextblocks } = insights
  const toneOf = (tone: string): 'text' | 'secondary' | 'tertiary' | 'danger' | 'attention' | 'accent' | 'added' | 'removed' => {
    if (tone === 'danger' || tone === 'error') return 'danger'
    if (tone === 'accent') return 'accent'
    if (tone === 'good') return 'added'
    if (tone === 'warn') return 'attention'
    if (tone === 'tertiary') return 'tertiary'
    return 'secondary'
  }
  return (
    <VStack gap={12}>
      <CardSection title={tr('Context blocks')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(contextblocks.tone)}>{contextblocks.headline}</Text>
        <ValueRow label={tr('Blocks')}>{String(contextblocks.blocks)}</ValueRow>
        <ValueRow label={tr('Turns')}>{String(contextblocks.turns)}</ValueRow>
        <ValueRow label={tr('No reply')}>{contextblocks.noReply ? tr('Yes') : tr('No')}</ValueRow>
      </CardSection>

      <CardSection title={tr('Work')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(work.tone)}>{work.headline}</Text>
        <ValueRow label={tr('Calls')}>{String(work.calls)}</ValueRow>
        <ValueRow label={tr('Duration')}>{work.duration}</ValueRow>
        <ValueRow label={tr('State')}>{work.state}</ValueRow>
      </CardSection>

      <CardSection title={tr('Channel access')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(channelaccess.tone)}>{channelaccess.headline}</Text>
        <ValueRow label={tr('Summary')}>{channelaccess.summaryLine}</ValueRow>
        <ValueRow label={tr('Pairing')}>{channelaccess.pairing ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Provider')}>{channelaccess.provider}</ValueRow>
      </CardSection>

      <CardSection title={tr('Layout')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(layout.tone)}>{layout.headline}</Text>
        <ValueRow label={tr('Panes')}>{String(layout.panes)}</ValueRow>
        <ValueRow label={tr('Active')}>{layout.active}</ValueRow>
        <ValueRow label={tr('Orientation')}>{layout.orientation}</ValueRow>
      </CardSection>

      <CardSection title={tr('Draft stash')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(draftstash.tone)}>{draftstash.headline}</Text>
        <ValueRow label={tr('Entries')}>{String(draftstash.entries)}</ValueRow>
        <ValueRow label={tr('Title')}>{draftstash.title}</ValueRow>
        <ValueRow label={tr('Stamp')}>{String(draftstash.stamp)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Service')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(service.tone)}>{service.headline}</Text>
        <ValueRow label={tr('Unit')}>{service.unit}</ValueRow>
        <ValueRow label={tr('Drift')}>{service.drift ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Running')}>{service.running}</ValueRow>
      </CardSection>

      <CardSection title={tr('Plugins')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(plugins.tone)}>{plugins.headline}</Text>
        <ValueRow label={tr('Root')}>{plugins.root}</ValueRow>
        <ValueRow label={tr('API version')}>{plugins.apiVersion}</ValueRow>
        <ValueRow label={tr('Compatible')}>{plugins.compatible ? tr('Yes') : tr('No')}</ValueRow>
      </CardSection>

      <CardSection title={tr('Terminal sequences')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(csiterm.tone)}>{csiterm.headline}</Text>
        <ValueRow label={tr('Prefix')}>{JSON.stringify(csiterm.prefix)}</ValueRow>
        <ValueRow label={tr('Cursor style')}>{JSON.stringify(csiterm.cursorStyle)}</ValueRow>
        <ValueRow label={tr('Scroll')}>{JSON.stringify(csiterm.scroll)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Prompt actions')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(promptactions.tone)}>{promptactions.headline}</Text>
        <ValueRow label={tr('Timeout')}>{promptactions.timeout ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Surface')}>{promptactions.surface ?? '—'}</ValueRow>
        <ValueRow label={tr('In flight')}>{promptactions.inFlight ? tr('Yes') : tr('No')}</ValueRow>
      </CardSection>

      <CardSection title={tr('Bot store')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(botstore.tone)}>{botstore.headline}</Text>
        <ValueRow label={tr('Valid')}>{botstore.valid ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Folder')}>{botstore.folder}</ValueRow>
        <ValueRow label={tr('Deliverable')}>{botstore.deliverable}</ValueRow>
      </CardSection>

      <CardSection title={tr('Model preferences')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(modelprefs.tone)}>{modelprefs.headline}</Text>
        <ValueRow label={tr('Default')}>{modelprefs.modelDefault}</ValueRow>
        <ValueRow label={tr('Chat')}>{modelprefs.chat ?? '—'}</ValueRow>
        <ValueRow label={tr('Levels')}>{modelprefs.levels.join(' · ')}</ValueRow>
      </CardSection>

      <CardSection title={tr('Router')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(router.tone)}>{router.headline}</Text>
        <ValueRow label={tr('Selected')}>{router.selected.join(', ') || '—'}</ValueRow>
        <ValueRow label={tr('Everyone')}>{router.everyone ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Strategy')}>{router.strategy}</ValueRow>
      </CardSection>

      <CardSection title={tr('Command policy')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(cmdpolicy.tone)}>{cmdpolicy.headline}</Text>
        <ValueRow label={tr('Verdict')}>{cmdpolicy.verdict}</ValueRow>
        <ValueRow label={tr('Denied')}>{cmdpolicy.denied ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Read only')}>{cmdpolicy.readOnly ? tr('Yes') : tr('No')}</ValueRow>
      </CardSection>

      <CardSection title={tr('Missions')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(missions.tone)}>{missions.headline}</Text>
        <ValueRow label={tr('Cells')}>{missions.cells.join(' · ')}</ValueRow>
        <ValueRow label={tr('Span')}>{missions.span}</ValueRow>
        <ValueRow label={tr('Open')}>{missions.open ? tr('Yes') : tr('No')}</ValueRow>
      </CardSection>

      <CardSection title={tr('No AI')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(noai.tone)}>{noai.headline}</Text>
        <ValueRow label={tr('State')}>{noai.state}</ValueRow>
        <ValueRow label={tr('Brain')}>{noai.brain}</ValueRow>
        <ValueRow label={tr('Choices')}>{String(noai.choices)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Devices')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(devices.tone)}>{devices.headline}</Text>
        <ValueRow label={tr('All')}>{String(devices.all)}</ValueRow>
        <ValueRow label={tr('Look')}>{devices.look}</ValueRow>
        <ValueRow label={tr('Link')}>{devices.link}</ValueRow>
      </CardSection>

      <CardSection title={tr('Vault')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(vault.tone)}>{vault.headline}</Text>
        <ValueRow label={tr('Variable')}>{vault.variable}</ValueRow>
        <ValueRow label={tr('Title')}>{vault.title}</ValueRow>
        <ValueRow label={tr('Policy')}>{vault.policy}</ValueRow>
      </CardSection>

      <CardSection title={tr('Tool replay')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(toolreplay.tone)}>{toolreplay.headline}</Text>
        <ValueRow label={tr('Kept')}>{String(toolreplay.kept)}</ValueRow>
        <ValueRow label={tr('Replay')}>{toolreplay.replay ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Outcome')}>{toolreplay.outcome}</ValueRow>
      </CardSection>

      <CardSection title={tr('Secret cleanup')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(secretcleanup.tone)}>{secretcleanup.headline}</Text>
        <ValueRow label={tr('Kind')}>{secretcleanup.kind}</ValueRow>
        <ValueRow label={tr('Retry due')}>{String(secretcleanup.retryDue)}</ValueRow>
        <ValueRow label={tr('Backup due')}>{String(secretcleanup.backupDue)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Avatar')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(avatar.tone)}>{avatar.headline}</Text>
        <ValueRow label={tr('Hex')}>{avatar.hex}</ValueRow>
        <ValueRow label={tr('Eye')}>{avatar.eye}</ValueRow>
        <ValueRow label={tr('Contrast')}>{avatar.contrast}</ValueRow>
      </CardSection>

      <CardSection title={tr('Providers')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(providers.tone)}>{providers.headline}</Text>
        <ValueRow label={tr('Keys')}>{String(providers.keys)}</ValueRow>
        <ValueRow label={tr('Usable models')}>{String(providers.usable)}</ValueRow>
        <ValueRow label={tr('Guess')}>{providers.guess}</ValueRow>
      </CardSection>

      <CardSection title={tr('Coding plans')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(plans.tone)}>{plans.headline}</Text>
        <ValueRow label={tr('Provider plan')}>{plans.providerPlan ?? '—'}</ValueRow>
        <ValueRow label={tr('Choice')}>{plans.choice}</ValueRow>
        <ValueRow label={tr('Key plan')}>{plans.keyPlan ? tr('Yes') : tr('No')}</ValueRow>
      </CardSection>

      <CardSection title={tr('Accounts')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(accounts.tone)}>{accounts.headline}</Text>
        <ValueRow label={tr('State')}>{accounts.state}</ValueRow>
        <ValueRow label={tr('Action')}>{accounts.action}</ValueRow>
        <ValueRow label={tr('Title')}>{accounts.title}</ValueRow>
      </CardSection>

      <CardSection title={tr('Engine')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(engine.tone)}>{engine.headline}</Text>
        <ValueRow label={tr('Engine')}>{engine.engine}</ValueRow>
        <ValueRow label={tr('State')}>{engine.state}</ValueRow>
        <ValueRow label={tr('Line')}>{engine.line}</ValueRow>
      </CardSection>

      <CardSection title={tr('Brain picker')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(brainpicker.tone)}>{brainpicker.headline}</Text>
        <ValueRow label={tr('Groups')}>{brainpicker.groups.join(', ')}</ValueRow>
        <ValueRow label={tr('Free')}>{String(brainpicker.free)}</ValueRow>
        <ValueRow label={tr('Keywords')}>{brainpicker.keywords}</ValueRow>
      </CardSection>

      <CardSection title={tr('Approval rules')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(approvalrules.tone)}>{approvalrules.headline}</Text>
        <ValueRow label={tr('Tool')}>{approvalrules.tool}</ValueRow>
        <ValueRow label={tr('SQL read')}>{approvalrules.sqlRead ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Rules')}>{String(approvalrules.rules)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Iteration')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(iteration.tone)}>{iteration.headline}</Text>
        <ValueRow label={tr('Ghosts')}>{String(iteration.ghosts)}</ValueRow>
        <ValueRow label={tr('Tools')}>{String(iteration.tools)}</ValueRow>
        <ValueRow label={tr('Restart')}>{iteration.restartLabel}</ValueRow>
      </CardSection>

      <CardSection title={tr('Recurrence')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(rrule.tone)}>{rrule.headline}</Text>
        <ValueRow label={tr('Cron')}>{rrule.cron ?? '—'}</ValueRow>
        <ValueRow label={tr('Weekdays')}>{rrule.weekdays.join(', ')}</ValueRow>
        <ValueRow label={tr('Timezone')}>{rrule.timezone}</ValueRow>
      </CardSection>

      <CardSection title={tr('Autofill')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(autofill.tone)}>{autofill.headline}</Text>
        <ValueRow label={tr('OTP')}>{String(autofill.otp)}</ValueRow>
        <ValueRow label={tr('Checkout')}>{String(autofill.checkout)}</ValueRow>
        <ValueRow label={tr('Best')}>{autofill.best}</ValueRow>
      </CardSection>

      <CardSection title={tr('Verification')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(verify.tone)}>{verify.headline}</Text>
        <ValueRow label={tr('Evidence')}>{verify.evidence}</ValueRow>
        <ValueRow label={tr('Paths')}>{String(verify.paths)}</ValueRow>
        <ValueRow label={tr('Action')}>{verify.nudgeLabel}</ValueRow>
      </CardSection>

      <CardSection title={tr('Window pin')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(contextpin.tone)}>{contextpin.headline}</Text>
        <ValueRow label={tr('Marker')}>{contextpin.marker}</ValueRow>
        <ValueRow label={tr('Pinned')}>{contextpin.pinned ? tr('Yes') : tr('No')}</ValueRow>
        <ValueRow label={tr('Ratio')}>{`${contextpin.ratio}%`}</ValueRow>
      </CardSection>

      <CardSection title={tr('Markdown tables')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(mdtables.tone)}>{mdtables.headline}</Text>
        <ValueRow label={tr('Rows')}>{String(mdtables.rows)}</ValueRow>
        <ValueRow label={tr('Layout')}>{mdtables.layout}</ValueRow>
        <ValueRow label={tr('Aligned')}>{String(mdtables.aligned)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Context budget')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(contextselect.tone)}>{contextselect.headline}</Text>
        <ValueRow label={tr('Available')}>{String(contextselect.available)}</ValueRow>
        <ValueRow label={tr('Kept')}>{String(contextselect.kept)}</ValueRow>
        <ValueRow label={tr('Cache')}>{contextselect.cache}</ValueRow>
      </CardSection>

      <CardSection title={tr('Usage picker')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(picker.tone)}>{picker.headline}</Text>
        <Text size="footnote" tone="secondary">{picker.bar}</Text>
        <ValueRow label={tr('Used')}>{picker.used}</ValueRow>
        <ValueRow label={tr('Resets')}>{picker.resets}</ValueRow>
      </CardSection>

      <CardSection title={tr('Command palette')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(palette.tone)}>{palette.headline}</Text>
        <ValueRow label={tr('Command')}>{palette.command}</ValueRow>
        <ValueRow label={tr('Results')}>{String(palette.results)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Shortcuts')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(shortcuts.tone)}>{shortcuts.headline}</Text>
        <ValueRow label={tr('Modifier')}>{shortcuts.modifier}</ValueRow>
        <ValueRow label={tr('Quote')}>{shortcuts.quote}</ValueRow>
        <ValueRow label={tr('Command menu')}>{shortcuts.commandMenu ?? '—'}</ValueRow>
      </CardSection>

      <CardSection title={tr('Eval metrics')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(evalextra.tone)}>{evalextra.headline}</Text>
        <ValueRow label={tr('Median')}>{String(evalextra.median)}</ValueRow>
        <ValueRow label={tr('Over 1k')}>{String(evalextra.over1k)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Connections extra')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(connextra.tone)}>{connextra.headline}</Text>
        <ValueRow label={tr('Channels')}>{connextra.channels.join(', ')}</ValueRow>
        <ValueRow label={tr('Selected')}>{String(connextra.selected)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Ranked search')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(searchextra.tone)}>{searchextra.headline}</Text>
        <ValueRow label={tr('Query')}>{searchextra.query}</ValueRow>
        <ValueRow label={tr('Matched')}>{String(searchextra.matched)}</ValueRow>
        <ValueRow label={tr('Sort')}>{searchextra.sort}</ValueRow>
      </CardSection>

      <CardSection title={tr('Memory context')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(memoryctx.tone)}>{memoryctx.headline}</Text>
        <Text size="footnote" tone="secondary">{memoryctx.preview}</Text>
        <ValueRow label={tr('Notes')}>{String(memoryctx.notes)}</ValueRow>
        <ValueRow label={tr('Hidden')}>{String(memoryctx.hidden)}</ValueRow>
      </CardSection>

      <CardSection title={tr('API errors')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(apierror.tone)}>{apierror.headline}</Text>
        <Text size="footnote" tone="secondary">{apierror.preview}</Text>
        <ValueRow label={tr('Class')}>{apierror.classLabel}</ValueRow>
        <ValueRow label={tr('Retryable')}>{apierror.retryable ? tr('Yes') : tr('No')}</ValueRow>
      </CardSection>

      <CardSection title={tr('History')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(history.tone)}>{history.headline}</Text>
        <Text size="footnote" tone="secondary">{history.preview}</Text>
        <ValueRow label={tr('Commentary')}>{String(history.commentary)}</ValueRow>
        <ValueRow label={tr('Withheld')}>{String(history.withheld)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Review queue')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(review.tone)}>{review.headline}</Text>
        <ValueRow label={tr('Mode')}>{review.mode}</ValueRow>
        <ValueRow label={tr('Queued')}>{String(review.queued)}</ValueRow>
        <ValueRow label={tr('Dispatchable')}>{String(review.dispatchable.length)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Blueprint')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(blueprint.tone)}>{blueprint.headline}</Text>
        <ValueRow label={tr('Cron')}>{blueprint.cron}</ValueRow>
        <ValueRow label={tr('Schedule')}>{blueprint.schedule}</ValueRow>
        <ValueRow label={tr('Command')}>{blueprint.slash}</ValueRow>
      </CardSection>

      <CardSection title={tr('Stream')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(stream.tone)}>{stream.headline}</Text>
        <ValueRow label={tr('Held')}>{String(stream.held)}</ValueRow>
        <ValueRow label={tr('Consumers')}>{String(stream.consumers)}</ValueRow>
        <ValueRow label={tr('Chunks')}>{String(stream.chunks)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Locale')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(i18n.tone)}>{i18n.headline}</Text>
        <ValueRow label={tr('Intl')}>{i18n.intl}</ValueRow>
        <ValueRow label={tr('Week starts')}>{i18n.weekLabel}</ValueRow>
        <ValueRow label={tr('Plural')}>{i18n.plural}</ValueRow>
      </CardSection>

      <CardSection title={tr('Quota')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(quota.tone)}>{quota.headline}</Text>
        <ValueRow label={tr('Status')}>{quota.status}</ValueRow>
        <ValueRow label={tr('Remaining')}>{`${quota.remaining}%`}</ValueRow>
        <ValueRow label={tr('Resets')}>{quota.resetPhrase}</ValueRow>
      </CardSection>

      <CardSection title={tr('Appearance')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(appearance.tone)}>{appearance.headline}</Text>
        <ValueRow label={tr('Appearance')}>{appearance.appearance}</ValueRow>
        <ValueRow label={tr('Accent')}>{appearance.accent}</ValueRow>
        <ValueRow label={tr('Body size')}>{String(appearance.bodySize)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Connections')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(connstate.tone)}>{connstate.headline}</Text>
        <ValueRow label={tr('State')}>{connstate.state}</ValueRow>
        <ValueRow label={tr('Connected')}>{String(connstate.connected)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Evaluation')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(evalResult.tone)}>{evalResult.headline}</Text>
        <ValueRow label={tr('Verdict')}>{evalResult.verdict}</ValueRow>
        <ValueRow label={tr('Score')}>{String(evalResult.score)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Search')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(search.tone)}>{search.headline}</Text>
        <ValueRow label={tr('Query')}>{search.query}</ValueRow>
        <ValueRow label={tr('Hits')}>{String(search.hits)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Mail & data')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(mailtext.tone)}>{mailtext.headline}</Text>
        <ValueRow label={tr('Address')}>{mailtext.address}</ValueRow>
        <ValueRow label={tr('Rows')}>{String(mailtext.rows)}</ValueRow>
        <ValueRow label={tr('Keys in text')}>{String(mailtext.keys)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Notifications')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(notify.tone)}>{notify.headline}</Text>
        <ValueRow label={tr('State')}>{notify.state}</ValueRow>
        <ValueRow label={tr('Diagnostic')}>{String(notify.diagnostic)}</ValueRow>
        <ValueRow label={tr('Failed')}>{String(notify.failed)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Markdown')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(markdown.tone)}>{markdown.headline}</Text>
        <Text size="footnote" tone="secondary">{markdown.preview}</Text>
        <ValueRow label={tr('Blocks')}>{String(markdown.blocks)}</ValueRow>
        <ValueRow label={tr('Mentions')}>{String(markdown.mentions)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Local format')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(localfmt.tone)}>{localfmt.headline}</Text>
        <ValueRow label={tr('Clock')}>{localfmt.clock}</ValueRow>
        <ValueRow label={tr('Weekday')}>{localfmt.weekday}</ValueRow>
        <ValueRow label={tr('Tokens')}>{localfmt.tokens}</ValueRow>
        <ValueRow label={tr('Budget')}>{localfmt.budget}</ValueRow>
      </CardSection>

      <CardSection title={tr('Text hygiene')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(textfmt.tone)}>{textfmt.headline}</Text>
        <ValueRow label={tr('Handle')}>{textfmt.handle}</ValueRow>
        <ValueRow label={tr('Invalid emails')}>{String(textfmt.invalidEmails)}</ValueRow>
        <ValueRow label={tr('Hidden marks')}>{String(textfmt.hidden)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Context budget')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(agenteschat.windowTone)}>{agenteschat.headline}</Text>
        <ValueRow label={tr('Window')}>{agenteschat.window}</ValueRow>
        <ValueRow label={tr('Mode')}>{agenteschat.auto}</ValueRow>
      </CardSection>

      <CardSection title={tr('Run model')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(openbot.tone)}>{openbot.headline}</Text>
        <ValueRow label={tr('Effort')}>{openbot.effort}</ValueRow>
        <ValueRow label={tr('Route')}>{openbot.route}</ValueRow>
      </CardSection>

      <CardSection title={tr('Learning')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(opendots.learningTone)}>{opendots.headline}</Text>
        <ValueRow label={tr('Container')}>{opendots.container}</ValueRow>
        <ValueRow label={tr('Internal messages')}>{String(opendots.internal)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Context window')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(context.tone)}>{context.headline}</Text>
        <ValueRow label={tr('Page states')}>{context.page}</ValueRow>
        <ValueRow label={tr('Screenshots')}>{context.screenshots}</ValueRow>
        <Text size="footnote" tone="tertiary">{context.pageNote}</Text>
      </CardSection>

      <CardSection title={tr('Liveness')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(liveness.tone)}>{liveness.headline}</Text>
        <ValueRow label={tr('State')}>{liveness.state}</ValueRow>
        <ValueRow label={tr('Delivery')}>{liveness.outcome}</ValueRow>
      </CardSection>

      <CardSection title={tr('Voice dictation')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(dictation.tone)}>{dictation.headline}</Text>
        <ValueRow label={tr('Engine')}>{dictation.engine}</ValueRow>
        <ValueRow label={tr('Model')}>{dictation.model}</ValueRow>
        <ValueRow label={tr('Language')}>{dictation.language}</ValueRow>
      </CardSection>

      <CardSection title={tr('Public access')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(access.modeTone)}>{access.mode}</Text>
        <ValueRow label={tr('Routing')}>{access.routing}</ValueRow>
        {access.collections.length ? (
          <VStack gap={2}>
            {access.collections.map((item) => (
              <HStack key={item.name} gap={6}>
                <Text size="footnote" tone={item.valid ? 'text' : 'danger'}>{item.name}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Last message')}>
        <Text size="subheadline" weight="semibold">{blocks.headline}</Text>
        <Text size="footnote" tone="tertiary">{blocks.origin}</Text>
        {blocks.blocks.map((item, i) => (
          <VStack key={`${item.kind}-${i}`} gap={0}>
            <Text size="footnote" tone={toneOf(item.tone)}>{`${item.says ? '●' : '○'} ${item.text}`}</Text>
          </VStack>
        ))}
      </CardSection>

      <CardSection title={tr('Directory')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(directory.tone)}>{directory.headline}</Text>
        <Text size="footnote" tone="accent">{directory.bar}</Text>
        <ValueRow label={tr('Directory')}>{directory.directory}</ValueRow>
        <ValueRow label={tr('Integrations')}>{String(directory.integrations.length)}</ValueRow>
        {directory.integrations.map((item) => (
          <Text key={item.id} size="footnote" tone="secondary">{`${item.name} · ${item.header}`}</Text>
        ))}
      </CardSection>

      <CardSection title={tr('Prompt style')}>
        <Text size="subheadline" weight="semibold">{prompts.model}</Text>
        <Text size="footnote" tone="secondary">{prompts.price}</Text>
        <Text size="footnote" tone="tertiary">{`${prompts.context} · ${prompts.zone} · ${prompts.length}`}</Text>
        {prompts.lines.map((line, i) => (
          <Text key={i} size="footnote" tone="secondary">{line}</Text>
        ))}
      </CardSection>

      <CardSection title={tr('Latest from bdots')}>
        <Text size="subheadline" weight="semibold">{fresh.agent}</Text>
        {fresh.summary.map((row) => (
          <HStack key={row.key} gap={6}>
            <Text size="footnote" tone="tertiary">{row.key}</Text>
            <Text size="footnote" tone="secondary">{row.value}</Text>
          </HStack>
        ))}
        <Text size="footnote" tone="tertiary">{`${fresh.titles.exec} · ${fresh.titles.write}`}</Text>
      </CardSection>

      <CardSection title={tr('Agent profile')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(agent.tone)}>{agent.label}</Text>
        <Text size="footnote" tone="secondary">{agent.headline}</Text>
        {agent.lines.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.set ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.label}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Conversation')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(conversation.tone)}>{conversation.headline}</Text>
        <Text size="footnote" tone="secondary">{conversation.line}</Text>
        {conversation.kinds.map((kind) => (
          <HStack key={kind.kind} gap={6}>
            <Text size="footnote" tone={toneOf(kind.tone)}>{kind.label}</Text>
            <Text size="footnote" tone="accent">{kind.bar}</Text>
            <Text size="footnote" tone="secondary">{String(kind.count)}</Text>
          </HStack>
        ))}
      </CardSection>

      {failures.total !== '0' ? (
        <CardSection title={tr('Recent failures')}>
          <Text size="subheadline" weight="semibold" tone={toneOf(failures.tone)}>{failures.headline}</Text>
          {failures.categories.map((cat) => (
            <HStack key={cat.kind} gap={6}>
              <Text size="footnote" tone={toneOf(cat.tone)}>{cat.kind}</Text>
              <Text size="footnote" tone="accent">{cat.bar}</Text>
              <Text size="footnote" tone="secondary">{String(cat.count)}</Text>
            </HStack>
          ))}
          {failures.items.map((item, i) => (
            <VStack key={`${item.name}-${i}`} gap={0}>
              <Text size="footnote" tone={toneOf(item.tone)}>{item.name}</Text>
              <Text size="footnote" tone="tertiary">{`${item.reason} · ${item.age}`}</Text>
            </VStack>
          ))}
        </CardSection>
      ) : null}

      <CardSection title={tr('Approvals')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(approvals.tone)}>{approvals.headline}</Text>
        <Text size="footnote" tone="accent">{approvals.bar}</Text>
        {approvals.items.map((item) => (
          <VStack key={item.id} gap={0}>
            <Text size="footnote" tone={toneOf(item.tone)}>{item.title}</Text>
            <Text size="footnote" tone="tertiary">{`${item.detail} · ${item.age}`}</Text>
          </VStack>
        ))}
      </CardSection>

      <CardSection title={tr('Time zones')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(timezone.tone)}>{timezone.headline}</Text>
        <ValueRow label={tr('This computer')}>{timezone.local}</ValueRow>
        {timezone.all.map((zone) => (
          <HStack key={zone.name} gap={6}>
            <Text size="footnote" tone={toneOf(zone.tone)}>{zone.label}</Text>
            <Text size="footnote" tone="accent">{zone.bar}</Text>
            <Text size="footnote" tone="secondary">{String(zone.count)}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('At a glance')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(digest.tone)}>{digest.label}</Text>
        <Text size="footnote" tone="secondary">{digest.headline}</Text>
        <Text size="footnote" tone="tertiary">{digest.oneLine}</Text>
        {digest.stats.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.tone)}>{item.label}</Text>
            <Text size="footnote" tone="accent">{item.value}</Text>
          </HStack>
        ))}
      </CardSection>

      {insights.description ? (
        <CardSection title={tr('About')}>
          <Text size="footnote" tone="secondary">{insights.description}</Text>
        </CardSection>
      ) : null}

      <CardSection title={tr('Model')}>
        <ValueRow label={tr('Agent')}>{insights.provider ?? tr('Unknown')}</ValueRow>
        {insights.model ? <ValueRow label={tr('Model')}>{insights.model}</ValueRow> : null}
      </CardSection>

      <CardSection title={tr('Today')}>
        <ValueRow label={tr('Tokens')}>
          {usage.limit === null ? usage.usedText : `${usage.usedText} / ${usage.limit}`}
        </ValueRow>
        <ValueRow label={tr('Skills')}>{String(skills.total)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Suggestions')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(advice.tone)}>{advice.label}</Text>
        <Text size="footnote" tone="secondary">{advice.headline}</Text>
        {advice.items.map((item) => (
          <VStack key={item.id} gap={0}>
            <Text size="footnote" tone={toneOf(item.priorityTone)}>{`${item.priorityLabel}: ${item.title}`}</Text>
            <Text size="footnote" tone="tertiary">{item.detail}</Text>
          </VStack>
        ))}
      </CardSection>

      <CardSection title={tr('Session')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(session.tone)}>{session.label}</Text>
        <Text size="footnote" tone="secondary">{session.headline}</Text>
        <Text size="footnote" tone="accent">{session.bar}</Text>
        <ValueRow label={tr('Session')}>{session.session}</ValueRow>
        <ValueRow label={tr('Unread')}>{session.unread}</ValueRow>
        {session.lines.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.ok ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.label}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Workspace')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(workspace.tone)}>{workspace.label}</Text>
        <Text size="footnote" tone="secondary">{workspace.headline}</Text>
        <ValueRow label={tr('Folder')}>{workspace.folder}</ValueRow>
        {workspace.lines.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.set ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.label}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Memory')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(memory.tone)}>{memory.label}</Text>
        <Text size="footnote" tone="secondary">{memory.headline}</Text>
        <ValueRow label={tr('Shared notes')}>{memory.shared}</ValueRow>
        <Text size="footnote" tone="tertiary">{memory.sharedDetail}</Text>
        {memory.lines.map((item) => (
          <HStack key={item.id + item.title} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.on ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.title}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Model tuning')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(tuning.tone)}>{tuning.effort}</Text>
        <Text size="footnote" tone="secondary">{tuning.headline}</Text>
        <ValueRow label={tr('Options')}>{tuning.count}</ValueRow>
        {tuning.enabled.length ? (
          <VStack gap={2}>
            {tuning.enabled.map((opt) => (
              <HStack key={opt.title} gap={6}>
                <Text size="footnote" tone="tertiary">{opt.title}</Text>
                <Text size="footnote" tone="accent">{opt.value}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {tuning.options.length ? (
          <VStack gap={2}>
            {tuning.options.map((opt) => (
              <VStack key={opt.key} gap={0}>
                <Text size="footnote" tone={toneOf(opt.tone)}>{`${opt.title}: ${opt.value}`}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Connected accounts')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(connectors.tone)}>{connectors.headline}</Text>
        {connectors.categories.length ? (
          <VStack gap={2}>
            {connectors.categories.map((cat) => (
              <HStack key={cat.kind} gap={6}>
                <Text size="footnote" tone={toneOf(cat.tone)}>{cat.kind}</Text>
                <Text size="footnote" tone="accent">{cat.bar}</Text>
                <Text size="footnote" tone="secondary">{String(cat.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {connectors.connectors.length ? (
          <VStack gap={2}>
            {connectors.connectors.map((item) => (
              <VStack key={item.name} gap={0}>
                <Text size="footnote" tone={toneOf(item.tone)}>{item.name}</Text>
                <Text size="footnote" tone="tertiary">{item.detail}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Identity')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(identity.tone)}>{identity.headline}</Text>
        <ValueRow label={tr('Roster')}>{identity.band}</ValueRow>
        <ValueRow label={tr('Section')}>{identity.section}</ValueRow>
        <ValueRow label={tr('Rank')}>{String(identity.rank)}</ValueRow>
        <Text size="footnote" tone="tertiary">{`${identity.color} · ${identity.shape} · ${identity.look}`}</Text>
        {identity.traits.length ? (
          <VStack gap={2}>
            {identity.traits.map((trait) => (
              <HStack key={trait.id} gap={6}>
                <Text size="footnote" tone={toneOf(trait.stateTone)}>{trait.on ? '●' : '○'}</Text>
                <Text size="footnote" tone={toneOf(trait.stateTone)}>{`${trait.label}: ${trait.detail}`}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Resilience')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(resilience.tone)}>{resilience.label}</Text>
        <Text size="footnote" tone="accent">{resilience.bar}</Text>
        <Text size="footnote" tone="secondary">{resilience.headline}</Text>
        {resilience.switches.length ? (
          <HStack gap={8}>
            {resilience.switches.map((s) => (
              <Text key={s.label} size="footnote" tone={s.on ? 'added' : 'tertiary'}>
                {`${s.on ? '●' : '○'} ${s.label}`}
              </Text>
            ))}
          </HStack>
        ) : null}
        {resilience.fallbacks.length ? (
          <VStack gap={2}>
            {resilience.fallbacks.map((f) => (
              <VStack key={f.backend + f.model} gap={0}>
                <Text size="footnote" tone={toneOf(f.tone)}>{f.label}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Trust')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(trust.tone)}>{trust.label}</Text>
        <Text size="footnote" tone="secondary">{trust.headline}</Text>
        <ValueRow label={tr('Approval')}>{trust.modeLabel}</ValueRow>
        <Text size="footnote" tone="tertiary">{trust.modeDetail}</Text>
        {trust.rules.length ? (
          <VStack gap={2}>
            {trust.rules.map((rule) => (
              <VStack key={`${rule.pattern}-${rule.kind}`} gap={0}>
                <Text size="footnote" tone={toneOf(rule.tone)}>{`${rule.decision}: ${rule.pattern}`}</Text>
                {rule.kind ? <Text size="footnote" tone="tertiary">{rule.kind}</Text> : null}
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Skill library')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(library.tone)}>{library.headline}</Text>
        {library.kinds.length ? (
          <VStack gap={2}>
            {library.kinds.map((kind) => (
              <HStack key={kind.kind} gap={6}>
                <Text size="footnote" tone={toneOf(kind.tone)}>{kind.kind}</Text>
                <Text size="footnote" tone="accent">{kind.bar}</Text>
                <Text size="footnote" tone="secondary">{String(kind.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {library.skills.length ? (
          <VStack gap={2}>
            {library.skills.slice(0, 8).map((skill) => (
              <VStack key={skill.name} gap={0}>
                <Text size="footnote" tone={toneOf(skill.tone)}>{skill.name}</Text>
                <Text size="footnote" tone="tertiary">{skill.detail}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Schedule')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(routineSummary.tone)}>{routineSummary.headline}</Text>
        <Text size="footnote" tone="accent">{routineSummary.bar}</Text>
        <ValueRow label={tr('Active')}>{String(routineSummary.active)}</ValueRow>
        <ValueRow label={tr('Paused')}>{String(routineSummary.paused)}</ValueRow>
        <ValueRow label={tr('Triggers')}>{routineSummary.mix}</ValueRow>
        {routineSummary.soonest.length ? (
          <VStack gap={2}>
            {routineSummary.soonest.map((item) => (
              <HStack key={item.id} gap={6}>
                <Text size="footnote" tone={toneOf(item.stateTone)}>{item.enabled ? '●' : '○'}</Text>
                <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.name} · ${item.trigger}`}</Text>
                {item.nextIn ? <Text size="footnote" tone="tertiary">{item.nextIn}</Text> : null}
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('This week')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(week.tone)}>{week.verdict}</Text>
        <Text size="footnote" tone="secondary">{week.headline}</Text>
        <ValueRow label={tr('Steps')}>{week.totalLabel}</ValueRow>
        <ValueRow label={tr('Trend')}>{week.trend.label}</ValueRow>
        {week.peak ? (
          <Text size="footnote" tone="secondary">{tr('Busiest')}: {week.peak.label} · {week.peak.count}</Text>
        ) : null}
        {week.top.length ? (
          <VStack gap={2}>
            {week.top.map((day) => (
              <HStack key={day.label} gap={6}>
                <Text size="footnote" tone="tertiary">{day.label}</Text>
                <Text size="footnote" tone="accent">{day.bar}</Text>
                <Text size="footnote" tone="secondary">{String(day.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Capabilities')}>
        <Text size="footnote" tone="accent">{profile.bar}</Text>
        <Text size="subheadline" weight="semibold">{profile.headline}</Text>
        {profile.nextUp.length ? (
          <Text size="footnote" tone="secondary">
            {tr('Next up')}: {profile.nextUp.map((c) => c.title).join(' · ')}
          </Text>
        ) : null}
        {profile.caps.map((cap) => (
          <HStack key={cap.id} gap={6}>
            <Text size="footnote" tone={toneOf(cap.stateTone)}>{cap.enabled ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(cap.stateTone)}>{`${cap.title}: ${cap.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Health')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(health.tone)}>{health.label}</Text>
        <Text size="footnote" tone="secondary">{health.headline}</Text>
        {health.checks.length ? (
          <VStack gap={2}>
            {health.checks.map((item) => (
              <VStack key={item.id} gap={0}>
                <Text size="footnote" tone={toneOf(item.tone)}>{`${item.title}: ${item.detail}`}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Activity')}>
        <Text size="footnote" tone="secondary">{activity.headline}</Text>
        <ValueRow label={tr('Steps')}>{String(activity.steps)}</ValueRow>
        {activity.errors ? (
          <Text size="footnote" tone="danger">
            {tr('{count} failed').replace('{count}', String(activity.errors))}
          </Text>
        ) : null}
        {activity.groups.length ? (
          <HStack gap={8}>
            {activity.groups.slice(0, 6).map((group) => (
              <Text key={group.kind} size="footnote" tone={toneOf(group.tone)}>
                {`${group.count} ${group.label}`}
              </Text>
            ))}
          </HStack>
        ) : null}
        {activity.days.length ? (
          <VStack gap={2}>
            {activity.days.slice(-5).map((day) => (
              <HStack key={day.day} gap={6}>
                <Text size="footnote" tone="tertiary">{day.label}</Text>
                <Text size="footnote" tone="accent">{'█'.repeat(Math.max(1, day.width))}</Text>
                <Text size="footnote" tone="secondary">{String(day.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {activity.items.length ? (
          <VStack gap={2}>
            {activity.items.slice(0, 6).map((item, i) => (
              <VStack key={`${item.text}-${i}`} gap={0}>
                <Text size="footnote" tone={toneOf(item.status === 'error' ? 'danger' : item.tone)}>{item.text}</Text>
                {item.detail ? (
                  <Text size="footnote" tone="tertiary">{item.detail}</Text>
                ) : null}
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      {insights.recentWork.length ? (
        <CardSection title={tr('Recent work')}>
          {insights.recentWork.map((line, i) => (
            <Text key={i} size="footnote" tone="secondary">{line}</Text>
          ))}
        </CardSection>
      ) : null}

      <CardSection title={tr('Routines')}>
        {insights.routines.length === 0 ? (
          <Text size="footnote" tone="secondary">{tr('No routines yet.')}</Text>
        ) : (
          insights.routines.map((routine) => (
            <VStack key={routine.id} gap={2}>
              <HStack gap={6}>
                <Text size="subheadline" weight="semibold">{routine.name}</Text>
                {!routine.enabled ? <Text size="footnote" tone="secondary">{tr('Paused')}</Text> : null}
              </HStack>
              {routine.schedules.length ? (
                <Text size="footnote" tone="secondary">{routine.schedules.join(' · ')}</Text>
              ) : null}
              {routine.nextIn ? (
                <Text size="footnote" tone="tertiary">{tr('Next in {time}').replace('{time}', routine.nextIn)}</Text>
              ) : null}
            </VStack>
          ))
        )}
      </CardSection>
    </VStack>
  )
}
