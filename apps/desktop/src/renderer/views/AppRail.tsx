import type { ReactNode } from 'react'
import type { N8nState } from '@shared/models'
import { Icon } from '../components/Icon'
import { useTr } from '../lib/use-language'
import appIcon from '../../../resources/icon.png'
import './app-rail.css'

/** Where the rail's icons point. The panel shows the list of the one that is current. */
export type AppRailPage = 'bots' | 'inbox' | 'board' | 'routines' | 'canvas' | 'usage' | 'marketplace' | 'settings'

/** One icon in the rail: a square button with a tooltip and a selected state. */
function RailButton({ label, icon, current, onClick, children, status }: {
  label: string
  icon?: string
  current?: boolean
  onClick: () => void
  children?: ReactNode
  /** A dot under the icon: `running`, `starting`, `error` or `off`. */
  status?: string
}) {
  return (
    <button
      type="button"
      className="app-rail-button"
      data-current={current ? 'true' : undefined}
      aria-label={label}
      aria-current={current ? 'page' : undefined}
      title={label}
      onClick={onClick}
    >
      {children ?? <Icon name={icon ?? 'circle'} size={19} />}
      {status ? <span className={`app-rail-dot ${status}`} aria-hidden /> : null}
    </button>
  )
}

function n8nStatus(n8n: N8nState | null | undefined): string {
  if (n8n?.state === 'running') return 'running'
  if (n8n?.state === 'starting') return 'starting'
  if (n8n?.state === 'error') return 'error'
  return 'off'
}

/**
 * The window's icon rail, in the shape of OpenBot's: the app at the top, the
 * pages in the middle, and the account and settings at the bottom. The list
 * panel beside it changes with the page.
 */
export function AppRail({ active, onBots, onInbox, onBoard, onRoutines, onCanvas, onUsage, onN8n, onBrowser, onExtensions, onSettings, onAccount, accountOpen, update, account, n8n, inboxCount = 0 }: {
  active: AppRailPage
  onBots: () => void
  onInbox: () => void
  onBoard: () => void
  onRoutines: () => void
  onCanvas: () => void
  onUsage: () => void
  onN8n: () => void
  onBrowser: () => void
  onExtensions: () => void
  onSettings: () => void
  onAccount: () => void
  accountOpen: boolean
  update?: ReactNode
  account?: ReactNode
  n8n?: N8nState | null
  /** Approvals waiting: a small count on the Inbox icon. */
  inboxCount?: number
}) {
  const tr = useTr()
  return (
    <nav className="app-rail" aria-label={tr('Codync')}>
      <div className="app-rail-logo-wrap">
        <img className="app-rail-logo" src={appIcon} alt={tr('Codync')} draggable={false} />
      </div>
      <div className="app-rail-group">
        <RailButton label={tr('Bots')} icon="bubble.left.and.bubble.right" current={active === 'bots'} onClick={onBots} />
        <RailButton label={tr('Inbox')} icon="tray" current={active === 'inbox'} onClick={onInbox}>
          <Icon name="tray" size={19} />
          {inboxCount > 0 ? <span className="app-rail-badge">{inboxCount > 99 ? '99+' : inboxCount}</span> : null}
        </RailButton>
        <RailButton label={tr('Board')} icon="checklist" current={active === 'board'} onClick={onBoard} />
        <RailButton label={tr('Routines')} icon="arrow.clockwise" current={active === 'routines'} onClick={onRoutines} />
        <RailButton label={tr('Bot canvas')} icon="arrow.triangle.branch" current={active === 'canvas'} onClick={onCanvas} />
        <RailButton label={tr('Usage')} icon="gauge.with.dots.needle.33percent" current={active === 'usage'} onClick={onUsage} />
        <RailButton
          label={tr('n8n: the workflow editor, run by this computer')}
          icon="point.3.connected.trianglepath.dotted"
          onClick={onN8n}
          status={n8nStatus(n8n)}
        />
        <RailButton label={tr('Browser')} icon="globe" onClick={onBrowser} />
        <RailButton label={tr('Extensions')} icon="square.grid.2x2" current={active === 'marketplace'} onClick={onExtensions} />
      </div>
      <div className="app-rail-group app-rail-bottom">
        {update}
        <RailButton label={tr('Settings')} icon="gearshape" current={active === 'settings'} onClick={onSettings} />
        <button
          type="button"
          className={`app-rail-account ${accountOpen ? 'open' : ''}`}
          aria-label={tr('Open account menu for Account')}
          aria-expanded={accountOpen}
          title={tr('Account menu')}
          onClick={onAccount}
        >
          {account}
        </button>
      </div>
    </nav>
  )
}
