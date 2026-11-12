import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import type { Bot, TeamMember, TeamSettings } from '@shared/models'
import { BotAvatar } from '../../components/Avatar'
import { Button, IconButton, SegmentedChoice } from '../../components/Controls'
import { AnchoredMenu, ModalHeader } from '../../components/Overlay'
import { Icon } from '../../components/Icon'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import './team-structure.css'

const NODE_WIDTH = 236
const NODE_HEIGHT = 128
const LEVEL_GAP = 176
const SIDE_GAP = 72
const MIN_ZOOM = 0.5
const MAX_ZOOM = 1.6

/** Common roles, offered as chips; the field takes any other. */
const ROLE_PRESETS = [
  'Supervisor',
  'UI/UX',
  'Código',
  'Auditor',
  'QA',
  'Producto',
  'Datos',
  'Marketing',
  'Soporte',
]

/** The team settings a group starts with: everyone on the chart, the chief at the top. */
export function defaultTeam(members: string[], chiefBotId: string | null): TeamSettings {
  const lead = chiefBotId && members.includes(chiefBotId) ? chiefBotId : (members[0] ?? '')
  const rows = members.map((id, index): TeamMember => ({
    botId: id,
    role: '',
    responsibility: '',
    reportsTo: id === lead ? null : lead || null,
    x: 40 + index * (NODE_WIDTH + SIDE_GAP),
    y: id === lead ? 40 : 40 + LEVEL_GAP,
  }))
  return { mode: 'smart', voice: 'lead', members: rows }
}

/**
 * Keeps the chart in step with the group without forcing it into a shape: a member that left the
 * group leaves the chart, a line to someone who left becomes no line, and the chief stops reporting
 * to anyone. Everyone else stays exactly where the user put them, including several tops.
 */
export function reconcileTeam(team: TeamSettings | null | undefined, members: string[], chiefBotId: string | null): TeamSettings {
  const base = team ?? defaultTeam(members, chiefBotId)
  const lead = chiefBotId && members.includes(chiefBotId) ? chiefBotId : (members[0] ?? '')
  const rows = base.members
    .filter((m) => members.includes(m.botId))
    .map((m) => {
      if (m.botId === lead) return { ...m, reportsTo: null }
      if (m.reportsTo && !members.includes(m.reportsTo)) return { ...m, reportsTo: null }
      return m
    })
  return { ...base, members: rows }
}

/** Tree levels for the auto-layout; several tops are allowed. */
function layout(members: TeamMember[], lead: string): TeamMember[] {
  const by = new Map(members.map((m) => [m.botId, m]))
  const children = new Map<string, string[]>()
  for (const m of members) {
    const parent = m.reportsTo && by.has(m.reportsTo) ? m.reportsTo : null
    if (!parent) continue
    children.set(parent, [...(children.get(parent) ?? []), m.botId])
  }
  const placed = new Map<string, { x: number; y: number }>()
  let cursor = 0
  const visit = (id: string, depth: number) => {
    const kids = children.get(id) ?? []
    if (kids.length === 0) {
      placed.set(id, { x: cursor * (NODE_WIDTH + SIDE_GAP), y: depth * LEVEL_GAP })
      cursor += 1
      return
    }
    const first = cursor
    for (const kid of kids) visit(kid, depth + 1)
    const center = ((first + cursor - 1) / 2) * (NODE_WIDTH + SIDE_GAP)
    placed.set(id, { x: center, y: depth * LEVEL_GAP })
  }
  if (lead) visit(lead, 0)
  for (const m of members) if (!placed.has(m.botId)) visit(m.botId, 1)
  return members.map((m) => ({ ...m, x: placed.get(m.botId)?.x ?? m.x, y: placed.get(m.botId)?.y ?? m.y }))
}

/**
 * A group's team as a canvas: cards can be moved freely and connected by dragging from a card's
 * bottom port to the member it reports to. Several tops are allowed; the lead is the card marked
 * with the star. A role can be one of the presets or anything typed, and every card carries what
 * it is accountable for.
 */
export function TeamStructureEditor({ bots, chiefBotId, team, onChange, onLead }: {
  bots: Bot[]
  chiefBotId: string | null
  team: TeamSettings
  onChange: (team: TeamSettings) => void
  onLead: (botId: string | null) => void
}) {
  const tr = useTr()
  const lead = chiefBotId && bots.some((b) => b.id === chiefBotId) ? chiefBotId : (bots[0]?.id ?? null)
  const [selected, setSelected] = useState<string | null>(lead)
  const [dragging, setDragging] = useState<{ id: string; dx: number; dy: number } | null>(null)
  const [linking, setLinking] = useState<{ from: string; x: number; y: number } | null>(null)
  const [zoom, setZoom] = useState(1)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [adding, setAdding] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const surface = useRef<HTMLDivElement | null>(null)
  const addButton = useRef<HTMLButtonElement | null>(null)
  const panning = useRef<{ start: { x: number; y: number }; origin: { x: number; y: number } } | null>(null)

  const rows = useMemo(() => reconcileTeam(team, bots.map((b) => b.id), lead), [team, bots, lead])
  // Missing members are added by hand, so the chart can stay as the user shaped it.
  const missing = bots.filter((bot) => !rows.members.some((m) => m.botId === bot.id))
  useEffect(() => {
    if (team !== rows && (team.members.length !== rows.members.length || team.members.some((m, i) => m.botId !== rows.members[i]?.botId))) {
      onChange(rows)
    }
  }, [rows, team, onChange])

  const by = new Map(rows.members.map((m) => [m.botId, m]))
  const botOf = (id: string) => bots.find((b) => b.id === id)
  const update = (botId: string, patch: Partial<TeamMember>) =>
    onChange({ ...rows, members: rows.members.map((m) => (m.botId === botId ? { ...m, ...patch } : m)) })

  const setLead = (botId: string) => {
    onLead(botId)
    onChange({ ...rows, members: rows.members.map((m) => (m.botId === botId ? { ...m, reportsTo: null } : m)) })
  }

  const addMember = (botId: string) => {
    const spot = rows.members.reduce((acc, m) => ({ x: Math.max(acc.x, m.x + NODE_WIDTH + 40), y: Math.min(acc.y, m.y) }), { x: 40, y: 40 })
    onChange({ ...rows, members: [...rows.members, { botId, role: '', responsibility: '', reportsTo: lead, x: spot.x, y: spot.y }] })
    setSelected(botId)
    setAdding(false)
  }

  const removeFromChart = (botId: string) => {
    onChange({
      ...rows,
      members: rows.members
        .filter((m) => m.botId !== botId)
        .map((m) => (m.reportsTo === botId ? { ...m, reportsTo: null } : m)),
    })
    setSelected(null)
  }

  /** A line can't loop: the target can't already be below the source. */
  const canReportTo = (from: string, to: string): boolean => {
    if (from === to) return false
    let current: string | null | undefined = to
    for (let guard = 0; current && guard < 64; guard += 1) {
      if (current === from) return false
      current = by.get(current)?.reportsTo
    }
    return true
  }

  const worldPoint = (event: { clientX: number; clientY: number }) => {
    const rect = surface.current?.getBoundingClientRect()
    return rect ? { x: (event.clientX - rect.left - pan.x) / zoom, y: (event.clientY - rect.top - pan.y) / zoom } : { x: 0, y: 0 }
  }

  const startDrag = (event: ReactPointerEvent, id: string) => {
    const row = by.get(id)
    if (!row) return
    const p = worldPoint(event)
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    setDragging({ id, dx: p.x - row.x, dy: p.y - row.y })
    setSelected(id)
  }
  const startLink = (event: ReactPointerEvent, from: string) => {
    event.stopPropagation()
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    const p = worldPoint(event)
    setLinking({ from, x: p.x, y: p.y })
  }
  const onMove = (event: ReactPointerEvent) => {
    if (panning.current) {
      setPan({
        x: panning.current.origin.x + event.clientX - panning.current.start.x,
        y: panning.current.origin.y + event.clientY - panning.current.start.y,
      })
      return
    }
    if (linking) {
      const p = worldPoint(event)
      setLinking({ ...linking, x: p.x, y: p.y })
      return
    }
    if (!dragging) return
    const p = worldPoint(event)
    update(dragging.id, { x: Math.round(Math.max(0, p.x - dragging.dx)), y: Math.round(Math.max(0, p.y - dragging.dy)) })
  }
  const onUp = (event: ReactPointerEvent) => {
    if (panning.current) {
      panning.current = null
      return
    }
    if (linking) {
      const under = document.elementFromPoint(event.clientX, event.clientY) as HTMLElement | null
      const target = under?.closest('[data-bot-id]')?.getAttribute('data-bot-id') ?? null
      if (target && target !== linking.from) {
        if (canReportTo(linking.from, target)) update(linking.from, { reportsTo: target })
        else setNotice(tr('That line would create a loop.'))
      }
      setLinking(null)
      return
    }
    setDragging(null)
  }

  const bounds = rows.members.reduce(
    (acc, m) => ({ w: Math.max(acc.w, m.x + NODE_WIDTH + 120), h: Math.max(acc.h, m.y + NODE_HEIGHT + 160) }),
    { w: 720, h: 420 },
  )

  const selectedRow = selected ? by.get(selected) : null
  const selectedBot = selected ? botOf(selected) : null
  const zoomBy = (delta: number) => setZoom((z) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Number((z + delta).toFixed(2)))))

  return (
    <VStack gap={12} className="team-structure">
      <div className="team-mode-row">
        <SegmentedChoice
          selection={rows.mode === 'roundtable' ? 'roundtable' : rows.mode === 'team' ? 'team' : 'smart'}
          options={[
            { id: 'smart', label: tr('Automatic') },
            { id: 'team', label: tr('One voice') },
            { id: 'roundtable', label: tr('Round table') },
          ]}
          onChange={(mode) => onChange({ ...rows, mode: mode as TeamSettings['mode'] })}
        />
        <Spacer />
        <Text size="caption" tone="secondary">
          {rows.mode === 'roundtable'
            ? tr('Every member answers in turn.')
            : rows.mode === 'team'
              ? tr('The lead answers for the whole team.')
              : tr('Who answers follows the request.')}
        </Text>
      </div>
      {rows.mode === 'smart' ? (
        <Text size="caption" tone="tertiary">
          {tr('Write /boss for the lead, name a role ("UI/UX", "Auditor") for that member, or ask in general and everyone answers in this chat.')}
        </Text>
      ) : null}
      {rows.mode === 'team' ? (
        <div className="team-voice-row">
          <Text size="caption" tone="secondary">{tr('Who may speak')}</Text>
          <SegmentedChoice
            selection={rows.voice === '' ? 'lead' : rows.voice}
            options={[
              { id: 'lead', label: tr('Lead only') },
              { id: 'lead_and_hands', label: tr('Lead + named') },
              { id: 'all', label: tr('Everyone') },
            ]}
            onChange={(voice) => onChange({ ...rows, voice: voice as TeamSettings['voice'] })}
          />
        </div>
      ) : null}
      <div className="team-canvas-toolbar">
        <HStack gap={4} align="center" className="team-zoom">
          <IconButton title={tr('Zoom out')} icon="minus.circle" size={24} onClick={() => zoomBy(-0.1)} />
          <Text size="caption">{Math.round(zoom * 100)}%</Text>
          <IconButton title={tr('Zoom in')} icon="plus" size={24} onClick={() => zoomBy(0.1)} />
          <IconButton title={tr('Fit to window')} icon="square.grid.2x2" size={24} onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }) }} />
        </HStack>
        <span className="team-toolbar-divider" />
        <IconButton title={tr('Arrange the chart')} icon="arrow.triangle.branch" size={24} onClick={() => onChange({ ...rows, members: layout(rows.members, lead ?? '') })} />
        <button ref={addButton} type="button" className="team-add" onClick={() => setAdding((v) => !v)} disabled={missing.length === 0}>
          <Icon name="plus" size={12} />
          {tr('Add member')}
        </button>
        <Spacer />
        <Text size="caption" tone="tertiary">{tr('Drag the dot under a card onto the member it reports to.')}</Text>
      </div>
      {notice ? <Text size="caption" tone="danger">{notice}</Text> : null}

      <div className="team-canvas-wrap">
        <div
          ref={surface}
          className={`team-canvas ${linking ? 'linking' : ''}`}
          onPointerDown={(e) => {
            if ((e.target as HTMLElement).closest('.team-card, button, input, select')) return
            ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
            panning.current = { start: { x: e.clientX, y: e.clientY }, origin: pan }
          }}
          onPointerMove={onMove}
          onPointerUp={onUp}
          onPointerCancel={() => { setLinking(null); setDragging(null) }}
          onDoubleClick={(e) => {
            if ((e.target as HTMLElement).closest('.team-card, .team-canvas-toolbar')) return
            setAdding(true)
          }}
        >
          <div className="team-world" style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`, width: bounds.w, height: bounds.h }}>
            <svg className="team-edges" aria-hidden>
              {rows.members.map((m) => {
                const parent = m.reportsTo ? by.get(m.reportsTo) : null
                if (!parent) return null
                const x1 = m.x + NODE_WIDTH / 2
                const y1 = m.y
                const x2 = parent.x + NODE_WIDTH / 2
                const y2 = parent.y + NODE_HEIGHT
                const mid = (y1 + y2) / 2
                return <path key={m.botId} d={`M${x1},${y1} C${x1},${mid} ${x2},${mid} ${x2},${y2}`} />
              })}
              {linking ? (
                <path
                  className="live"
                  d={`M${(by.get(linking.from)?.x ?? 0) + NODE_WIDTH / 2},${(by.get(linking.from)?.y ?? 0) + NODE_HEIGHT} L${linking.x},${linking.y}`}
                />
              ) : null}
            </svg>
            {rows.members.map((m) => {
              const bot = botOf(m.botId)
              if (!bot) return null
              return (
                <article
                  key={m.botId}
                  data-bot-id={m.botId}
                  className="team-card"
                  data-selected={selected === m.botId ? 'true' : undefined}
                  data-lead={m.botId === lead ? 'true' : undefined}
                  style={{ left: m.x, top: m.y, width: NODE_WIDTH }}
                  onPointerDown={(e) => {
                    if ((e.target as HTMLElement).closest('.team-port, button, input, select')) return
                    startDrag(e, m.botId)
                  }}
                  onClick={() => setSelected(m.botId)}
                >
                  <div className="team-card-head">
                    <BotAvatar bot={bot} size={30} animated={false} />
                    <span className="team-card-name">{bot.name}</span>
                    <Spacer />
                    {m.botId === lead ? (
                      <span className="team-lead-badge" title={tr('Lead')}>
                        <Icon name="star.fill" size={12} />
                      </span>
                    ) : (
                      <IconButton title={tr('Make lead')} icon="star" size={22} onClick={() => setLead(m.botId)} />
                    )}
                  </div>
                  {m.role ? <span className="team-role-pill">{m.role}</span> : null}
                  <span className="team-card-responsibility">{m.responsibility || tr('No responsibility yet')}</span>
                  <button
                    type="button"
                    className="team-port"
                    title={tr('Drag to the member this one reports to')}
                    aria-label={tr('Reports to')}
                    onPointerDown={(e) => startLink(e, m.botId)}
                  />
                </article>
              )
            })}
          </div>
        </div>

        <aside className="team-inspector">
          <ModalHeader title={selectedBot?.name ?? tr('Select a card')} icon={selectedBot ? undefined : 'cursorarrow.motionlines'} />
          {selectedRow && selectedBot ? (
            <VStack gap={12}>
              <div className="team-inspector-label">{tr('Role')}</div>
              <div className="team-role-chips">
                {ROLE_PRESETS.map((role) => (
                  <button
                    key={role}
                    type="button"
                    className="team-role-chip"
                    data-on={selectedRow.role === role ? 'true' : undefined}
                    onClick={() => update(selectedRow.botId, { role: selectedRow.role === role ? '' : role })}
                  >
                    {tr(role)}
                  </button>
                ))}
              </div>
              <input
                className="team-inspector-field"
                value={selectedRow.role}
                placeholder={tr('Or type a role')}
                onChange={(e) => update(selectedRow.botId, { role: e.target.value })}
              />
              <div className="team-inspector-label">{tr('Responsibility')}</div>
              <textarea
                className="team-inspector-field area"
                value={selectedRow.responsibility}
                placeholder={tr('What it is accountable for')}
                onChange={(e) => update(selectedRow.botId, { responsibility: e.target.value })}
              />
              <div className="team-inspector-label">{tr('Reports to')}</div>
              <select
                className="team-inspector-field"
                value={selectedRow.reportsTo ?? ''}
                onChange={(e) => {
                  const to = e.target.value
                  if (to && !canReportTo(selectedRow.botId, to)) {
                    setNotice(tr('That line would create a loop.'))
                    return
                  }
                  update(selectedRow.botId, { reportsTo: to || null })
                }}
              >
                <option value="">{tr('Nobody (a top)')}</option>
                {rows.members
                  .filter((m) => m.botId !== selectedRow.botId && canReportTo(selectedRow.botId, m.botId))
                  .map((m) => (
                    <option key={m.botId} value={m.botId}>
                      {botOf(m.botId)?.name ?? m.botId}
                    </option>
                  ))}
              </select>
              <HStack gap={8}>
                {selectedRow.botId === lead ? (
                  <Text size="caption" tone="secondary">{tr('This is the lead.')}</Text>
                ) : (
                  <Button kind="secondary" onClick={() => setLead(selectedRow.botId)}>{tr('Make lead')}</Button>
                )}
                <Spacer />
                <IconButton title={tr('Remove from chart')} icon="xmark" onClick={() => removeFromChart(selectedRow.botId)} />
              </HStack>
            </VStack>
          ) : (
            <Text size="caption" tone="tertiary">{tr('Pick a card to edit its role, responsibility and manager.')}</Text>
          )}
        </aside>
      </div>

      <AnchoredMenu
        open={adding}
        onClose={() => setAdding(false)}
        anchor={addButton}
        items={() =>
          missing.length === 0
            ? [{ title: tr('Every member is on the chart'), action: () => setAdding(false) }]
            : missing.map((bot) => ({ title: `${tr('Add')} ${bot.name}`, action: () => addMember(bot.id) }))
        }
      />
    </VStack>
  )
}
