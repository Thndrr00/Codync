import { useState } from 'react'
import { pageOf } from '@shared/island'
import { usagePercent } from '../../lib/usage-dots'
import type { UsageProvider, UsageWindow } from '@shared/models'
import { Icon } from '../../components/Icon'
import { prefs, usePref } from '../../lib/prefs'
import { usePixelRatio } from '../../lib/theme'
import { useResetLine, useTr } from '../../lib/use-language'
import { UsageBar } from '../usage/UsageBar'
import { mascotDataURL } from '../usage/ProviderMascot'
import { providerTintCSS, resetDescription, tightest, usageStatus, usedText, windowReading, windowTitle } from '../usage/usage-model'
import type { IslandModel } from './island-model'

// Codync's own part of the island: the plan limits the menu bar already tracks (local usage
// only), as a meter in the header and a Usage tab.

/** A provider's character for the black island, whatever the system appearance. */
function Mascot({ provider, size }: { provider: UsageProvider; size: number }) {
  const [style] = usePref(prefs.usageIconStyle)
  const ratio = usePixelRatio()
  const src = mascotDataURL(provider, size, style, true, ratio)
  return src ? <img src={src} alt="" width={size} height={size} className="island-mascot" /> : null
}

/** The limit closest to running out, across providers. */
function tightestOverall(usage: UsageProvider[]): { provider: UsageProvider; window: UsageWindow } | null {
  let best: { provider: UsageProvider; window: UsageWindow } | null = null
  for (const provider of usage) {
    const window = tightest(provider)
    if (window && (!best || window.percent > best.window.percent)) best = { provider, window }
  }
  return best
}

/** Header: the tightest limit as its provider's character, a short bar and the percent; opens Usage. */
export function UsageMeter({ model }: { model: IslandModel }) {
  const tr = useTr()
  const line = useResetLine()
  const top = tightestOverall(model.summary.usage)
  if (!top) return null
  const { provider, window } = top
  const title = tr(windowTitle(window))
  const reset = resetDescription(window)
  const spoken = reset ? line(reset) : ''
  const used = usedText(window.percent, tr)
  return (
    <button
      className="island-meter"
      title={`${provider.name} · ${title} · ${used}${spoken ? ` · ${spoken}` : ''}`}
      aria-label={`${tr('Usage')}: ${provider.name} ${title} ${used}`}
      onClick={() => model.setTab('usage')}
    >
      <Mascot provider={provider} size={15} />
      <span className="island-meter-bar">
        <UsageBar percent={window.percent} tint={providerTintCSS(provider.id)} />
      </span>
      <span className="island-meter-value">{`${Math.round(usagePercent(window.percent))}%`}</span>
    </button>
  )
}

/**
 * The providers closest to a limit first, two to a page side by side with arrows between pages, each
 * window a bar with when it resets. The header meter's provider is always on the first page; ones with
 * notes only come last.
 */
export function UsageView({ model }: { model: IslandModel }) {
  const usage = [...model.summary.usage].sort((a, b) => (tightest(b)?.percent ?? -1) - (tightest(a)?.percent ?? -1))
  const tr = useTr()
  const [wanted, setPage] = useState(0)
  const { items, page, pages } = pageOf(usage, wanted)
  if (!usage.length) {
    return (
      <div className="island-card island-empty">
        <span className="island-empty-text">
          <span className="island-title">{tr('No usage yet')}</span>
          <span className="island-subtitle">{tr('Plan limits show up here once an agent reports them.')}</span>
        </span>
      </div>
    )
  }
  return (
    <div className="island-usage-pages">
      {pages > 1 ? (
        <div className="island-usage-navigation">
          <button className="island-tab" aria-label={tr('Previous usage providers')} disabled={page === 0} onClick={() => setPage(page - 1)}>
            <Icon name="chevron.left" size={12} />
          </button>
          <span aria-live="polite">{page + 1} / {pages}</span>
          <button className="island-tab" aria-label={tr('Next usage providers')} disabled={page === pages - 1} onClick={() => setPage(page + 1)}>
            <Icon name="chevron.right" size={12} />
          </button>
        </div>
      ) : null}
      <div key={page} className="island-row-cards island-swap">
        {items.map((provider) => <ProviderUsage key={provider.id} provider={provider} />)}
      </div>
    </div>
  )
}

function ProviderUsage({ provider }: { provider: UsageProvider }) {
  const tr = useTr()
  const status = usageStatus(provider)
  return (
    <div className="island-card island-usage">
      <div className="island-agent">
        <Mascot provider={provider} size={20} />
        <span className="island-agent-name truncate" title={provider.name}>{provider.name}</span>
      </div>
      <div className="island-usage-windows" tabIndex={0} role="region" aria-label={`${tr('Usage')}: ${provider.name}`}>
        {status ? <span className="island-usage-note">{tr(status)}</span> : null}
        {provider.windows.map((window) => <WindowUsage key={window.id} window={window} tint={providerTintCSS(provider.id)} />)}
      </div>
    </div>
  )
}

function WindowUsage({ window: w, tint }: { window: UsageWindow; tint: string }) {
  const tr = useTr()
  const line = useResetLine()
  const reset = resetDescription(w)
  return (
    <div className="island-usage-row">
      <span className="island-usage-title" title={w.label}>{tr(windowTitle(w))}</span>
      {w.text ? <span className="island-usage-note">{w.text}</span> : (
        <>
          <span className="island-usage-value">{windowReading(w, tr)}</span>
          <UsageBar percent={w.percent} tint={tint} />
          {reset ? <span className="island-usage-reset">{line(reset)}</span> : null}
        </>
      )}
    </div>
  )
}
