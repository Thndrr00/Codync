import type { IslandAction } from '../../shared/island-activity'
import type { IslandGit, IslandOperations, IslandResources, IslandRuntime } from '../../shared/island-operations'
import type { BotApp } from '../../shared/bdots-models'
import type { HostHealth } from '../../shared/health'
import type { TokenBudget } from '../../shared/models'
import type { SSHState } from '../../shared/ssh'
import { parseRef } from '../../shared/bot-ref.ts'
import { Observable } from '../lib/observable.ts'
import type { AppModel } from './app-model'
import type { ComputerPanelState } from './computer-panel-store'
import { islandThreadNavigation } from './island-thread-navigation.ts'

const text = (error: unknown) => error instanceof Error ? error.message : String(error)
const empty = (ref: string): IslandOperations => ({ ref, loading: false, actionError: null, errors: [], at: null, runtime: null, usage: null, budgets: [], health: null, fallback: null, compactions: [], switches: [], resources: null, git: null, apps: [], computer: null, hostUpdate: null, localComputer: false, browserOpen: null })
interface Dependencies {
  reveal(ref: string): void
  focusBrowser(): Promise<boolean>
  browserState(): Promise<boolean>
  sshState(): Promise<SSHState>
  connectSSH(id: string): void | Promise<void>
}
const defaults: Dependencies = {
  reveal: (ref) => window.codync.island.reveal(ref),
  focusBrowser: () => window.codync.browser.focusExisting(),
  browserState: () => window.codync.browser.existing(),
  sshState: () => window.codync.ssh.state(),
  connectSSH: (id) => window.codync.ssh.connect(id),
}
const ownedActions = new Set(['loadOperations', 'openOperationPanel', 'openOperationDocument', 'openOperationApp', 'selectWorktreeBot', 'openExistingBrowser', 'retryIslandSSH'])

/** Host reports and device UI intents; never switches a Git checkout, runs a tool, or installs an update. */
export class IslandOperationsStore extends Observable {
  private value: IslandOperations | undefined
  private owner: object | undefined
  private generation = 0
  private navigation = 0
  private alive = true
  private apps = new Map<string, BotApp>()
  private retrying = new Set<string>()
  private readonly app: AppModel
  private readonly deps: Dependencies
  constructor(app: AppModel, deps: Partial<Dependencies> = {}) { super(); this.app = app; this.deps = { ...defaults, ...deps } }

  snapshot() {
    const ref = this.value && parseRef(this.value.ref), store = ref && this.app.store(ref.computerId)
    if (!ref || !store || store !== this.owner || !this.value) return undefined
    const bot = store.bots.get(ref.botId)
    const step = bot?.agentStep
    return { ...this.value, localComputer: store === this.app.local,
      fallback: step ? { agent: step.agent, model: step.model, reason: step.reason, since: step.since } : null,
      compactions: store.allEntries(ref.botId).filter((entry) => entry.data.compaction).slice(-10).reverse().map((entry) => ({ id: entry.id, at: entry.createdAt, byHost: entry.data.compaction?.byHost === true })),
      switches: store.allEntries(ref.botId).filter((entry) => entry.data.agentSwitch).slice(-10).reverse().map((entry) => ({ id: entry.id, at: entry.createdAt, from: entry.data.agentSwitch!.from, to: entry.data.agentSwitch!.to, reason: entry.data.agentSwitch!.reason, back: entry.data.agentSwitch!.back })),
    }
  }
  dispose() { this.alive = false; ++this.generation; ++this.navigation; this.apps.clear() }
  refresh() { const ref = this.snapshot()?.ref; if (ref && !this.value?.loading) void this.load(ref) }
  reportError(error: unknown) { if (this.alive) this.fail(text(error)) }

  async load(key: string) {
    const ref = parseRef(key), store = ref && this.app.store(ref.computerId)
    if (!this.alive || !ref || !store || !store.bots.has(ref.botId)) return
    if (this.value?.ref === key && this.owner === store && this.value.loading) return
    if (this.value?.ref !== key || this.owner !== store) { this.value = empty(key); this.owner = store; this.apps.clear() }
    const generation = ++this.generation
    const current = () => this.alive && generation === this.generation && this.app.store(ref.computerId) === store
    this.value = { ...this.value!, loading: true, errors: [], localComputer: store === this.app.local }; this.changed()
    try {
      if (store.connection.kind !== 'online') throw new Error('Connect this computer to load operations.')
      const client = await store.ready()
      if (!current()) return
      const results = await Promise.allSettled([
        client.call<IslandRuntime>('islandRuntime', { botId: ref.botId }),
        client.call<NonNullable<IslandOperations['usage']>>('botUsage', { botId: ref.botId }),
        client.call<{ budgets: TokenBudget[] }>('tokenBudgets'),
        client.call<HostHealth>('hostHealth'),
        client.call<IslandResources>('islandResources', { botId: ref.botId }),
        client.call<IslandGit>('islandGit', { botId: ref.botId }),
        client.call<{ apps: BotApp[] }>('listBotApps', { botId: ref.botId }),
        client.call<ComputerPanelState>('computerPanel', { botId: ref.botId }),
        store.isLoopback ? client.call<{ currentVersion: string; state: { phase: string; availableVersion: string | null; error: string | null } }>('hostUpdateStatus') : Promise.resolve(null),
        store === this.app.local ? Promise.resolve().then(() => this.deps.browserState()) : Promise.resolve(null),
      ] as const)
      if (!current()) return
      const errors: IslandOperations['errors'] = []
      const read = <T,>(index: number, section: string, fallback: T): T => {
        const result = results[index]!
        if (result.status === 'fulfilled') return result.value as T
        errors.push({ section, message: text(result.reason) }); return fallback
      }
      const runtime = read<IslandRuntime | null>(0, 'runtime', null)
      const usage = read<IslandOperations['usage']>(1, 'usage', null)
      const budgets = read(2, 'budgets', { budgets: [] as TokenBudget[] })
      const health = read<HostHealth | null>(3, 'health', null)
      const resources = read<IslandResources | null>(4, 'resources', null)
      const git = read<IslandGit | null>(5, 'git', null)
      const apps = read(6, 'apps', { apps: [] as BotApp[] })
      const computer = read<ComputerPanelState | null>(7, 'computer', null)
      const update = read<{ currentVersion: string; state: { phase: string; availableVersion: string | null; error: string | null } } | null>(8, 'hostUpdate', null)
      const browserOpen = read<boolean | null>(9, 'browser', null)
      for (const [section, value] of [['runtime', runtime], ['resources', resources], ['git', git]] as const) if (value?.error) errors.push({ section, message: value.error })
      this.apps = new Map(apps.apps.filter((item) => item.botId === ref.botId).slice(0, 40).map((item) => [item.slug, item]))
      this.value = {
        ...this.value!, loading: false, at: Date.now(), errors, browserOpen,
        runtime: runtime ? { context: runtime.context ? { used: runtime.context.used, size: runtime.context.size, lastTurn: runtime.context.lastTurn, compactions: runtime.context.compactions } : null, cost: { amount: null, currency: null, reason: 'notReported' }, mcp: runtime.mcp.slice(0, 20).map((call) => ({ id: call.id, connectorId: call.connectorId, tool: call.tool, status: call.status, createdAt: call.createdAt })), error: runtime.error } : null,
        usage: usage ? { used: usage.used, limit: usage.limit, fleetUsed: usage.fleetUsed, fleetLimit: usage.fleetLimit, resetsAt: usage.resetsAt } : null,
        budgets: budgets.budgets.filter((budget) => budget.botId === ref.botId || budget.botId === '*').map((budget) => ({ botId: budget.botId, limit: budget.limit, used: budget.used, level: budget.level })),
        health: health ? { cpu: health.cpu, ramUsedPercent: health.ramUsedPercent, ramFree: health.ramFree, ramTotal: health.ramTotal, diskFree: health.diskFree, diskTotal: health.diskTotal, at: health.at, rss: health.agents.find((agent) => agent.botId === ref.botId)?.rss ?? null } : null,
        resources: resources ? { documents: resources.documents.slice(0, 10).map((document) => ({ id: document.id, title: document.title, updatedAt: document.updatedAt })), channelFailures: resources.channelFailures.slice(0, 20).map((receipt) => ({ id: receipt.id, source: receipt.source, state: receipt.state })), channelState: resources.channelState.map((state) => ({ source: state.source, state: state.state })), scope: 'computer', error: resources.error } : null,
        git: git ? { root: git.root, conflicts: git.conflicts.slice(0, 100), worktrees: git.worktrees.slice(0, 20).map((tree) => ({ path: tree.path, branch: tree.branch, head: tree.head, detached: tree.detached, locked: tree.locked, prunable: tree.prunable, botIds: tree.botIds.slice(0, 100) })), error: git.error } : null,
        apps: [...this.apps.values()].map((item) => ({ slug: item.slug, title: item.title, version: item.version, updatedAt: item.updatedAt })),
        computer: computer ? { enabled: computer.enabled, userControl: computer.screen.userControl, agentBot: computer.screen.agentBot ?? null, computerUse: computer.screen.computerUse } : null,
        hostUpdate: update ? { currentVersion: update.currentVersion, phase: update.state.phase, availableVersion: update.state.availableVersion, error: update.state.error } : null,
      }
    } catch (error) { if (current()) { this.apps.clear(); this.value = { ...empty(key), localComputer: store === this.app.local, errors: [{ section: 'connection', message: text(error) }] } } }
    finally { if (current()) { this.value = { ...this.value!, loading: false }; this.changed() } }
  }

  async perform(action: IslandAction) {
    if (!ownedActions.has(action.kind) || !this.alive) return
    if (action.kind === 'retryIslandSSH') return this.retrySSH(action.id)
    if (!('ref' in action)) return
    if (action.kind === 'loadOperations') return this.load(action.ref)
    const ref = parseRef(action.ref), store = ref && this.app.store(ref.computerId)
    if (!ref || !store || !store.bots.has(ref.botId) || this.snapshot()?.ref !== action.ref || this.owner !== store) return
    if (action.kind === 'openExistingBrowser') {
      if (store !== this.app.local) { this.fail('The existing desktop browser belongs to this computer only.'); return }
      try { if (!await this.deps.focusBrowser() && this.alive && this.owner === store && this.value?.ref === action.ref) this.fail('No browser window is open. No session was created.') }
      catch (error) { if (this.alive && this.owner === store && this.value?.ref === action.ref) this.fail(text(error)) }
      return
    }
    if (action.kind === 'selectWorktreeBot') {
      if (!this.value!.git?.worktrees.some((tree) => tree.botIds.includes(action.id)) || !store.bots.has(action.id)) return
      ++this.navigation
      this.app.select({ computerId: ref.computerId, botId: action.id }); this.deps.reveal(`${ref.computerId}/${action.id}`); return
    }
    if (action.kind === 'openOperationPanel') {
      ++this.navigation
      if (action.panel === 'budget') islandThreadNavigation.open({ kind: 'budget', owner: store, ref: action.ref })
      else islandThreadNavigation.open({ kind: 'operationPanel', owner: store, ref: action.ref, panel: action.panel })
    } else if (action.kind === 'openOperationDocument') {
      if (!this.value!.resources?.documents.some((document) => document.id === action.id)) return
      const documents = store.documentsFor(ref.botId)
      if (documents.busy || documents.mutating) { this.fail('Wait for the document save to finish before opening another document.'); return }
      this.value = { ...this.value!, actionError: null }; this.changed()
      const navigation = ++this.navigation
      const current = () => this.alive && this.app.store(ref.computerId) === store && this.owner === store && this.value?.ref === action.ref && navigation === this.navigation
      try {
        documents.resume()
        await documents.open(action.id)
        if (!current()) return
        if (documents.error || documents.draft?.id !== action.id) { this.fail(documents.error ?? 'The requested document could not be opened.'); return }
      } catch (error) { if (current()) this.fail(text(error)); return }
      islandThreadNavigation.open({ kind: 'operationPanel', owner: store, ref: action.ref, panel: 'documents', documentId: action.id })
    } else if (action.kind === 'openOperationApp') {
      const app = this.apps.get(action.id)
      if (!app) return
      ++this.navigation
      islandThreadNavigation.open({ kind: 'miniApp', owner: store, ref: action.ref, app })
    } else return
    this.app.select(ref); this.deps.reveal(action.ref)
  }

  private fail(message: string) { if (this.value) this.value = { ...this.value, actionError: message }; this.changed() }
  private async retrySSH(id: string) {
    if (this.retrying.has(id)) return
    this.retrying.add(id)
    try {
      const state = await this.deps.sshState()
      if (!this.alive || !state.profiles.some((profile) => profile.id === id)) return
      const kind = state.status[id]?.kind ?? 'idle'
      if (kind !== 'idle' && kind !== 'failed' && kind !== 'retrying') { this.fail('Open Computers to finish this SSH connection.'); return }
      await this.deps.connectSSH(id)
    } catch (error) { if (this.alive) this.fail(text(error)) }
    finally { this.retrying.delete(id) }
  }
}
