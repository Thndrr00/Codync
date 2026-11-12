import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import type { Bot, TeamMember, TeamSettings } from '@shared/models'
import { BotAvatar } from '../../components/Avatar'
import { IconButton, SegmentedChoice } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import './team-structure.css'

const NODE_WIDTH = 236
const NODE_HEIGHT = 116
const LEVEL_GAP = 158
const SIDE_GAP = 56

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
  return { mode: 'team', voice: 'lead', members: rows }
}

/** Keeps the chart in step with the group's members and its chief. */
export function reconcileTeam(team: TeamSettings | null | undefined, members: string[], chiefBotId: string | null): TeamSettings {
  const base = team ?? defaultTeam(members, chiefBotId)
  const lead = chiefBotId && members.includes(chiefBotId) ? chiefBotId : (members[0] ?? '')
  const kept = base.members.filter((m) => members.includes(m.botId))
  const missing = members.filter((id) => !kept.some((m) => m.botId === id))
  let next = [...kept, ...missing.map((id, i) => ({ botId: id, role: '', responsibility: '', reportsTo: lead || null, x: 40 + (kept.length + i) * (NODE_WIDTH + SIDE_GAP), y: 40 + LEVEL_GAP }))]
  next = next.map((m) => (m.botId === lead ? { ...m, reportsTo: null } : m))
  // A reporting line to someone who left the group falls back to the lead.
  next = next.map((m) => (m.reportsTo && !members.includes(m.reportsTo) ? { ...m, reportsTo: lead || null } : m))
  return { ...base, members: next }
}

/** Tree levels for the auto-layout: the lead at the top, each report one level below its manager. */
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
 * A group's team as an org chart: each member a card with its role, who it reports to, and where
 * it sits. The lead is the group's chief; a card can report to any other member, and the chart
 * refuses a line that would loop.
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
  const surface = useRef<HTMLDivElement | null>(null)

  const rows = useMemo(() => reconcileTeam(team, bots.map((b) => b.id), lead), [team, bots, lead])
  useEffect(() => {
    if (team.members.length !== rows.members.length || team.members.some((m, i) => m.botId !== rows.members[i]?.botId)) onChange(rows)
    // Only membership is reconciled here; edits stay as the user made them.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows.members.length])

  const by = new Map(rows.members.map((m) => [m.botId, m]))
  const botOf = (id: string) => bots.find((b) => b.id === id)
  const update = (botId: string, patch: Partial<TeamMember>) =>
    onChange({ ...rows, members: rows.members.map((m) => (m.botId === botId ? { ...m, ...patch } : m)) })

  const setLead = (botId: string) => {
    onLead(botId)
    onChange({
      ...rows,
      members: rows.members.map((m) => (m.botId === botId ? { ...m, reportsTo: null } : m.reportsTo === botId ? { ...m, reportsTo: null } : m)),
    })
  }

  // A reporting line can't loop: the target and its own chain can't report back to this member.
  const canReportTo = (from: string, to: string): boolean => {
    if (from === to) return false
    let current: string | null | undefined = to
    for (let guard = 0; current && guard < 32; guard += 1) {
      if (current === from) return false
      current = by.get(current)?.reportsTo
    }
    return true
  }

  const startDrag = (event: ReactPointerEvent, id: string) => {
    const rect = surface.current?.getBoundingClientRect()
    const row = by.get(id)
    if (!rect || !row) return
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    setDragging({ id, dx: event.clientX - rect.left - row.x, dy: event.clientY - rect.top - row.y })
    setSelected(id)
  }
  const moveDrag = (event: ReactPointerEvent) => {
    if (!dragging) return
    const rect = surface.current?.getBoundingClientRect()
    if (!rect) return
    update(dragging.id, {
      x: Math.round(Math.max(0, event.clientX - rect.left - dragging.dx)),
      y: Math.round(Math.max(0, event.clientY - rect.top - dragging.dy)),
    })
  }

  const bounds = rows.members.reduce(
    (acc, m) => ({ w: Math.max(acc.w, m.x + NODE_WIDTH + 80), h: Math.max(acc.h, m.y + NODE_HEIGHT + 80) }),
    { w: 640, h: 360 },
  )

  const selectedRow = selected ? by.get(selected) : null
  const selectedBot = selected ? botOf(selected) : null

  return (
    <VStack gap={12} className="team-structure">
      <div className="team-mode-row">
        <SegmentedChoice
          selection={rows.mode === 'roundtable' ? 'roundtable' : 'team'}
          options={[
            { id: 'team', label: tr('One voice') },
            { id: 'roundtable', label: tr('Round table') },
          ]}
          onChange={(mode) => onChange({ ...rows, mode: mode as TeamSettings['mode'] })}
        />
        <Spacer />
        <Text size="caption" tone="secondary">
          {rows.mode === 'roundtable' ? tr('Every member answers in turn.') : tr('The lead answers for the whole team.')}
        </Text>
      </div>

      {rows.mode !== 'roundtable' ? (
        <>
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

          <div className="team-toolbar">
            <HStack gap={6} align="center">
              <IconButton
                title={tr('Arrange the chart')}
                icon="square.grid.2x2"
                onClick={() => onChange({ ...rows, members: layout(rows.members, lead ?? '') })}
              />
              <Text size="caption" tone="tertiary">{tr('Drag a card to move it. Lines are who reports to whom.')}</Text>
            </HStack>
            <Spacer />
            {selectedRow ? (
              <HStack gap={6} align="center">
                <Text size="caption" tone="secondary">{selectedRow.botId === lead ? tr('Lead') : tr('Reports to')}</Text>
                {selectedRow.botId === lead ? null : (
                  <select
                    className="team-select"
                    value={selectedRow.reportsTo ?? ''}
                    onChange={(e) => {
                      const to = e.target.value
                      if (to && !canReportTo(selectedRow.botId, to)) return
                      update(selectedRow.botId, { reportsTo: to || null })
                    }}
                  >
                    <option value="">{tr('Nobody')}</option>
                    {rows.members
                      .filter((m) => m.botId !== selectedRow.botId && canReportTo(selectedRow.botId, m.botId))
                      .map((m) => (
                        <option key={m.botId} value={m.botId}>
                          {botOf(m.botId)?.name ?? m.botId}
                        </option>
                      ))}
                  </select>
                )}
              </HStack>
            ) : null}
          </div>

          <div className="team-chart" ref={surface} onPointerMove={moveDrag} onPointerUp={() => setDragging(null)} onPointerCancel={() => setDragging(null)}>
            <svg className="team-chart-edges" width="100%" height="100%" aria-hidden>
              {rows.members.map((m) => {
                const parent = m.reportsTo ? by.get(m.reportsTo) : null
                if (!parent) return null
                const x1 = parent.x + NODE_WIDTH / 2
                const y1 = parent.y + NODE_HEIGHT
                const x2 = m.x + NODE_WIDTH / 2
                const y2 = m.y
                const mid = (y1 + y2) / 2
                return <path key={m.botId} d={`M${x1},${y1} C${x1},${mid} ${x2},${mid} ${x2},${y2}`} />
              })}
            </svg>
            <div className="team-chart-world" style={{ width: bounds.w, height: bounds.h }}>
              {rows.members.map((m) => {
                const bot = botOf(m.botId)
                if (!bot) return null
                return (
                  <article
                    key={m.botId}
                    className="team-card"
                    data-selected={selected === m.botId ? 'true' : undefined}
                    data-lead={m.botId === lead ? 'true' : undefined}
                    style={{ left: m.x, top: m.y, width: NODE_WIDTH }}
                    onPointerDown={(e) => {
                      if ((e.target as HTMLElement).closest('.team-card-field, button, select')) return
                      startDrag(e, m.botId)
                    }}
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
                    <input
                      className="team-card-field role"
                      value={m.role}
                      placeholder={tr('Role, such as Research')}
                      onChange={(e) => update(m.botId, { role: e.target.value })}
                    />
                    <input
                      className="team-card-field responsibility"
                      value={m.responsibility}
                      placeholder={tr('What it is accountable for')}
                      onChange={(e) => update(m.botId, { responsibility: e.target.value })}
                    />
                  </article>
                )
              })}
            </div>
          </div>

          {selectedBot ? (
            <Text size="caption" tone="tertiary">
              {tr('Selected')}: {selectedBot.name} · {selectedBot.description || tr('No description')}
            </Text>
          ) : null}
        </>
      ) : (
        <Text size="caption" tone="secondary">
          {tr('The round table keeps the classic behaviour: members answer one at a time, and you can @mention one.')}
        </Text>
      )}
    </VStack>
  )
}
