import { useEffect, useRef, useState } from 'react'
import type { IslandBot, IslandPermission, IslandSummary } from '@shared/island'
import { folderName, isGroup, isWorking, type Bot, type Entry } from '@shared/models'
import { useModel, useModels } from '../lib/observable'
import { IslandIntegrationsStore } from '../store/island-integrations-store'
import { islandSignals } from '../store/island-signals'
import { activeDictation } from './thread/active-dictation'
import { stopDictation } from './thread/dictation'
import { IslandWorkspaceStore } from '../store/island-workspace-store'
import { IslandActivityStore } from '../store/island-activity-store'
import { activeCall } from './call/active-call'
import { refKey, type AppModel, type RosterItem } from '../store/app-model'
import type { BotStore } from '../store/bot-store'
import { healthAlert } from '@shared/health'
import { botDetail } from './tray-summary'
import { optionLabel, permissionHeadline, sortedOptions } from './thread/PermissionCard'
import { trNow } from '../lib/l10n.ts'

/** Tool calls in the island's ticker. */
const STEPS = 3

/**
 * Sends the notch island what it shows when the stores change: at most every 150 ms while
 * replies stream (reading the stores when it fires), and only when that changed.
 */
export function useIslandSummary(app: AppModel) {
  const [activity] = useState(() => new IslandActivityStore(app))
  const [workspace] = useState(() => new IslandWorkspaceStore(app))
  const [integrations] = useState(() => new IslandIntegrationsStore(app))
  useModel(integrations)
  useModel(activeDictation)
  useModel(workspace)
  const sent = useRef('')
  const pending = useRef<ReturnType<typeof setTimeout> | null>(null)
  useModel(activity)
  useModel(activeCall)
  useModels([...app.stores.values()])
  useEffect(() => {
    activity.start()
    const off = window.codync.app.onCommand((command) => {
      if (command.kind === 'islandAction') { void activity.perform(command.action); void workspace.perform(command.action); void integrations.perform(command.action); if (command.action.kind === 'stopDictation') stopDictation(command.action.id) }
    })
    const integrationsTimer = setInterval(() => { void window.codync.island.enabled().then((enabled) => { if (enabled) integrations.refresh() }).catch((error) => integrations.reportPollingError(error)) }, 60_000)
    const timer = setInterval(() => workspace.refresh(), 15_000)
    return () => { clearInterval(integrationsTimer); integrations.dispose(); clearInterval(timer); workspace.dispose(); off(); activity.dispose(); if (pending.current) clearTimeout(pending.current); pending.current = null }
  }, [activity, workspace, integrations])
  const stores = [...app.stores.values()]
  useEffect(() => {
    const offs = stores.map((store) => store.composerDrafts.subscribe(() => activity.noteDrafts()))
    return () => offs.forEach((off) => off())
  })
  // The computer's health changes on its own: look again every so often.
  useEffect(() => {
    void app.local?.refreshHealth()
    const timer = setInterval(() => void app.local?.refreshHealth(), 15_000)
    return () => clearInterval(timer)
  }, [app.local])
  useEffect(() => {
    if (pending.current) return
    pending.current = setTimeout(() => {
      pending.current = null
      const summary = islandSummary(app, activity, workspace, integrations)
      const json = JSON.stringify(summary)
      if (sent.current === json) return
      sent.current = json
      window.codync.island.setSummary(summary)
    }, 150)
  })
}

function islandSummary(app: AppModel, activity: IslandActivityStore, workspace: IslandWorkspaceStore, integrations: IslandIntegrationsStore): IslandSummary {
  const many = app.computers.length > 1
  return {
    bots: app.roster
      .filter((i) => !isGroup(i.bot))
      .map((i) => islandBot(i.store, i.bot, refKey(i.ref), many ? `${i.bot.name} · ${i.computer.name}` : i.bot.name)),
    permissions: app.roster.flatMap(pendingCards),
    usage: activity.usage.providers,
    health: healthAlert(app.local?.health ?? null),
    activity: activity.snapshot(),
    workspace: workspace.snapshot(),
    integrations: integrations.snapshot(),
    signals: islandSignals(app),
  }
}

function islandBot(store: BotStore, bot: Bot, ref: string, name: string): IslandBot {
  const entries = store.allEntries(bot.id)
  return {
    ref,
    name,
    shape: bot.avatarShape,
    customIcon: bot.customIcon,
    color: bot.avatarColor,
    status: bot.status,
    backend: store.backendName(bot.backend),
    folder: folderName(bot),
    project: bot.cwd,
    hasDraft: !!store.composerDrafts.get(bot.id).trim(),
    startedAt: bot.startedAt,
    detail: botDetail(bot),
    steps: isWorking(bot)
      ? entries.filter((e) => e.kind === 'tool').slice(-STEPS).map((e) => ({ title: e.data.title || trNow('Use a tool'), kind: e.data.toolKind ?? 'other' }))
      : [],
    error: bot.status === 'error' ? (lastError(entries) ?? null) : null,
    unread: bot.unread,
  }
}

/** The text of the conversation's last error notice. */
const lastError = (entries: Entry[]) => entries.findLast((e) => e.kind === 'notice' && e.data.style === 'error')?.data.text

/** A conversation's unanswered approval cards; in a group, the member that asks. */
function pendingCards({ ref, bot, store }: RosterItem): IslandPermission[] {
  return store
    .allEntries(bot.id)
    .filter((e) => e.kind === 'permission' && e.data.status === 'pending')
    .map((e) => {
      const author = (e.data.author ? store.bots.get(e.data.author) : undefined) ?? bot
      return {
        ref: refKey(ref),
        createdAt: e.createdAt,
        entryId: e.id,
        bot: { customIcon: author.customIcon, name: author.name, shape: author.avatarShape, color: author.avatarColor, backend: store.backendName(author.backend) },
        headline: permissionHeadline(e.data.toolKind),
        title: e.data.title ?? '',
        // The island is small: the first files and their changed lines are enough to judge a card.
        diffs: (e.data.diffs ?? []).slice(0, 3).map((d) => ({ path: d.path, added: d.added, removed: d.removed, patch: d.patch })),
        options: sortedOptions(e.data.options ?? []).map((o) => ({ id: o.optionId, label: optionLabel(o), kind: o.kind })),
        answering: store.answering.get(e.id) ?? null,
      }
    })
}
