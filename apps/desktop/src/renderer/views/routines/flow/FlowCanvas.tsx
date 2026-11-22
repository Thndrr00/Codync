import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from 'react'
import type { Bot } from '@shared/models'
import { IconButton } from '../../../components/Controls'
import { Icon } from '../../../components/Icon'
import { MenuPanel, type MenuItem } from '../../../components/Menu'
import { useEscape, usePresence } from '../../../components/Overlay'
import { useTr } from '../../../lib/use-language'
import {
  NODE_HEIGHT, NODE_WIDTH, edgePath, fitView, isUnconnected, latestStep, linkEndOf, mediaOf, mediaTitle, nodeAt,
  portCarry, stepNameOf, stepOf, stepProgress, zoomAt, type FlowNode, type View,
} from '../../../lib/routine-flow.ts'
import {
  CanvasWithGroups, groupKey, groupOf, groupedEdges, groupedNodes, groupsOf, parsePort, portPrefix,
  type FlowGroup, type GroupedEdge,
} from './flow-groups'
import { FlowGroupCard, type GroupPreview } from './FlowGroupCard'
import { FlowNodeCard } from './FlowNodeCard'
import { flowMenuPoint } from './flow-settings'
import { MediaComposer } from './MediaComposer'
import type { FlowTarget } from './FlowInspector'

export type FlowSelection = { kind: 'node'; key: string } | { kind: 'edge'; id: string } | { kind: 'group'; id: string } | null

interface Props {
  canvas: CanvasWithGroups
  bots: ReadonlyMap<string, Bot>
  selection: FlowSelection
  disabled: boolean
  onSelect: (selection: FlowSelection) => void
  onMove: (nodeKey: string, x: number, y: number) => void
  /** A whole group (and its members) was dragged to a new origin. */
  onMoveGroup: (groupId: string, x: number, y: number) => void
  /** Puts the steps back in run-order columns (Shift+T, or the toolbar button). */
  onTidy: () => void
  /** Undoes a tidy by restoring the places it replaced; absent until one has run. */
  onUndoTidy?: () => void
  /** Runs this routine's flow now, from its card. */
  onRun: (routineId: string) => void
  /** Stops the routine run in flight, from its card (OpenBot 0.71.0). */
  onStop: (routineId: string) => void
  /** A step's connection was dropped on another bot or media step. */
  onConnect: (fromBotId: string, toBotId: string, at: { x: number; y: number }) => void
  /** Several selected nodes are being grouped (the view asks for the name). */
  onGroup: (keys: string[]) => void
  onUngroup: (groupId: string) => void
  /** Collapse or expand a group (double-click a stack card opens it). */
  onToggleGroup: (groupId: string) => void
  /** Two or more files were pasted or dropped; the view makes a node per file in one group. */
  onFiles: (files: { name: string }[]) => void
  /** The step menu's rows: Run from this step, Settings and (for a picture step) Duplicate. */
  menuItems: (nodeKey: string) => MenuItem[]
  /** Clicking a card's title edits its name; empty clears it back to the step type. */
  onRenameStep: (nodeKey: string, name: string) => void
  /** The canvas search; a node that doesn't match is dimmed. */
  query: string
  /** The composer under the selected media card, wired by the view that owns the host calls. */
  composer: {
    busy: boolean
    offline: boolean
    targets: (from: string) => FlowTarget[]
    onSaveMedia: (media: import('../../../lib/routine-flow.ts').FlowMedia, settings: import('./FlowInspector').MediaSettings) => void
    onRunFromStep: (nodeKey: string) => void
  }
}

const NUDGE = 20
const DRAG_SLOP = 3
const PORT_LABEL: Record<string, string> = { text: 'Reply', image: 'Picture', video: 'Clip' }
const PORT_ICON: Record<string, string> = { text: 'bubble.left', image: 'photo', video: 'film' }

/** A small overview of the canvas: every card, and the part the viewport shows. */
function MiniMap({ nodes, view, box, onCenter }: {
  nodes: FlowNode[]
  view: View
  box: { width: number; height: number }
  onCenter: (x: number, y: number) => void
}) {
  const tr = useTr()
  if (nodes.length === 0) return null
  const pad = 10
  const left = Math.min(...nodes.map((n) => n.x)) - pad
  const top = Math.min(...nodes.map((n) => n.y)) - pad
  const right = Math.max(...nodes.map((n) => n.x + NODE_WIDTH)) + pad
  const bottom = Math.max(...nodes.map((n) => n.y + NODE_HEIGHT)) + pad
  const scale = Math.min(150 / (right - left), 90 / (bottom - top))
  const offsetX = (150 - (right - left) * scale) / 2 - left * scale
  const offsetY = (90 - (bottom - top) * scale) / 2 - top * scale
  const at = (x: number, y: number) => ({ x: x * scale + offsetX, y: y * scale + offsetY })
  const visible = {
    x: at(-view.x / view.scale, 0).x,
    y: at(0, -view.y / view.scale).y,
    width: (box.width / view.scale) * scale,
    height: (box.height / view.scale) * scale,
  }
  return (
    <button
      type="button"
      className="flow-minimap"
      aria-label={tr('Canvas overview')}
      title={tr('Canvas overview')}
      onClick={(event) => {
        const rect = event.currentTarget.getBoundingClientRect()
        const px = ((event.clientX - rect.left) / rect.width) * 150
        const py = ((event.clientY - rect.top) / rect.height) * 90
        onCenter((px - offsetX) / scale + left, (py - offsetY) / scale + top)
      }}
    >
      <svg viewBox="0 0 150 90" aria-hidden="true">
        {nodes.map((node) => {
          const p = at(node.x, node.y)
          return <rect key={node.key} x={p.x} y={p.y} width={NODE_WIDTH * scale} height={NODE_HEIGHT * scale} rx={2} data-kind={node.key.startsWith('group:') ? 'group' : node.key.startsWith(portPrefix) ? 'media' : node.kind} />
        })}
        <rect className="flow-minimap-view" x={visible.x} y={visible.y} width={visible.width} height={visible.height} rx={2} />
      </svg>
    </button>
  )
}

/** The canvas: routine and bot nodes, their edges, groups, pan, zoom (wheel and buttons) and gestures. */
export function FlowCanvasView({ canvas, bots, selection, disabled, onSelect, onMove, onMoveGroup, onConnect, onGroup, onUngroup, onToggleGroup, onFiles, onTidy, onUndoTidy, onRun, onStop, menuItems, onRenameStep, query, composer }: Props) {
  const tr = useTr()
  const groups = groupsOf(canvas)
  const nodes = useMemo(() => groupedNodes(canvas), [canvas])
  const edges = useMemo(() => groupedEdges(canvas), [canvas])
  const [view, setView] = useState<View>({ x: 0, y: 0, scale: 1 })
  const [live, setLive] = useState<{ key: string; x: number; y: number } | null>(null)
  const [wire, setWire] = useState<{ from: string; x: number; y: number } | null>(null)
  const [multi, setMulti] = useState<Set<string>>(new Set())
  /** A step's menu, anchored to its button but kept inside the canvas (never under the tools). */
  const [menu, setMenu] = useState<{ key: string; x: number; top: number; bottom: number } | null>(null)
  const menuPresence = usePresence(menu !== null, 120)
  const menuLayer = useRef<HTMLDivElement>(null)
  useEscape(menu !== null, () => setMenu(null), menuLayer)
  const viewport = useRef<HTMLDivElement>(null)
  const [box, setBox] = useState({ width: 800, height: 500 })
  const fitted = useRef(false)
  const viewRef = useRef(view)
  viewRef.current = view

  useLayoutEffect(() => {
    const element = viewport.current
    if (!element) return
    const read = () => setBox({ width: element.clientWidth, height: element.clientHeight })
    read()
    const observer = new ResizeObserver(read)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  const size = () => viewport.current?.getBoundingClientRect() ?? new DOMRect(0, 0, 800, 500)
  useLayoutEffect(() => {
    if (fitted.current || nodes.length === 0) return
    fitted.current = true
    const { width, height } = size()
    setView(fitView(nodes, width, height))
  }, [nodes])

  // The wheel zooms around the pointer; it must not scroll the page behind, so it can't be a passive React handler.
  useEffect(() => {
    const element = viewport.current
    if (!element) return
    const wheel = (event: WheelEvent) => {
      event.preventDefault()
      const box = element.getBoundingClientRect()
      setView((v) => zoomAt(v, Math.exp(-event.deltaY * 0.0015), event.clientX - box.left, event.clientY - box.top))
    }
    element.addEventListener('wheel', wheel, { passive: false })
    return () => element.removeEventListener('wheel', wheel)
  }, [])

  const toCanvas = (clientX: number, clientY: number) => {
    const box = size()
    const v = viewRef.current
    return { x: (clientX - box.left - v.x) / v.scale, y: (clientY - box.top - v.y) / v.scale }
  }

  /** Follows the pointer until it lifts. */
  const track = (move: (event: globalThis.PointerEvent) => void, done: (event: globalThis.PointerEvent) => void) => {
    const up = (event: globalThis.PointerEvent) => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      done(event)
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
  }

  const startPan = (event: PointerEvent) => {
    if (event.target !== event.currentTarget && !(event.target as Element).classList.contains('flow-layer')) return
    onSelect(null)
    setMulti(new Set())
    setMenu(null)
    let last = { x: event.clientX, y: event.clientY }
    track(
      (e) => {
        const dx = e.clientX - last.x
        const dy = e.clientY - last.y
        last = { x: e.clientX, y: e.clientY }
        setView((v) => ({ ...v, x: v.x + dx, y: v.y + dy }))
      },
      () => {},
    )
  }

  const startDrag = (node: FlowNode, event: PointerEvent) => {
    if (event.button !== 0) return
    const origin = toCanvas(event.clientX, event.clientY)
    const grab = { x: origin.x - node.x, y: origin.y - node.y }
    let moved = false
    let at = { x: node.x, y: node.y }
    track(
      (e) => {
        const p = toCanvas(e.clientX, e.clientY)
        if (!moved && Math.hypot(p.x - origin.x, p.y - origin.y) * viewRef.current.scale < DRAG_SLOP) return
        moved = true
        at = { x: p.x - grab.x, y: p.y - grab.y }
        setLive({ key: node.key, ...at })
      },
      () => {
        setLive(null)
        if (moved) onMove(node.key, at.x, at.y)
      },
    )
  }

  /** Dragging a stack card moves the group's origin; the view shifts its members with it. */
  const startGroupDrag = (group: FlowGroup, event: PointerEvent) => {
    if (event.button !== 0) return
    const origin = toCanvas(event.clientX, event.clientY)
    const grab = { x: origin.x - group.x, y: origin.y - group.y }
    let moved = false
    let at = { x: group.x, y: group.y }
    track(
      (e) => {
        const p = toCanvas(e.clientX, e.clientY)
        if (!moved && Math.hypot(p.x - origin.x, p.y - origin.y) * viewRef.current.scale < DRAG_SLOP) return
        moved = true
        at = { x: p.x - grab.x, y: p.y - grab.y }
        setLive({ key: groupKey(group.id), ...at })
      },
      () => {
        setLive(null)
        if (moved) onMoveGroup(group.id, at.x, at.y)
      },
    )
  }

  const startWire = (from: string, event: PointerEvent) => {
    const start = toCanvas(event.clientX, event.clientY)
    setWire({ from, ...start })
    track(
      (e) => setWire({ from, ...toCanvas(e.clientX, e.clientY) }),
      (e) => {
        setWire(null)
        const p = toCanvas(e.clientX, e.clientY)
        const target = nodeAt([...byKey.values()], p.x, p.y)
        const valid = target && target.kind !== 'routine' && target.id !== from
          && !target.key.startsWith(portPrefix) && !target.key.startsWith('group:')
        if (valid) onConnect(from, target.id, { x: e.clientX, y: e.clientY })
      },
    )
  }

  const keyOnNode = (node: FlowNode, event: KeyboardEvent) => {
    const step = event.shiftKey ? NUDGE * 5 : NUDGE
    const delta: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }
    const d = delta[event.key]
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      onSelect(targetSelection(node.key))
    } else if (d && !disabled) {
      event.preventDefault()
      onMove(node.key, node.x + d[0], node.y + d[1])
    }
  }

  /** Group / ungroup from the keyboard, over any focused node or the canvas itself. */
  const canvasKeys = (event: KeyboardEvent) => {
    if (event.shiftKey && !event.ctrlKey && !event.metaKey && event.key.toLowerCase() === 't') {
      event.preventDefault()
      onTidy()
      return
    }
    if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'g') return
    event.preventDefault()
    if (event.shiftKey) {
      const id = selection?.kind === 'group' ? selection.id : groupOf(groups, [...multi][0] ?? '')?.id
      if (id) onUngroup(id)
    } else {
      const keys = multi.size >= 2 ? [...multi] : selection?.kind === 'node' ? [selection.key] : []
      if (keys.length >= 2) {
        onGroup(keys)
        setMulti(new Set())
      }
    }
  }

  const pick = (key: string, event: PointerEvent, id?: string) => {
    setMenu(null)
    if (event.ctrlKey || event.metaKey || event.shiftKey) {
      setMulti((prev) => {
        const next = new Set(prev)
        if (next.has(key)) next.delete(key)
        else next.add(key)
        return next
      })
    } else {
      setMulti(new Set([key]))
      onSelect(id ? { kind: 'group', id } : { kind: 'node', key })
    }
  }

  const targetSelection = (key: string) => key.startsWith('group:')
    ? { kind: 'group' as const, id: key.slice(6) }
    : { kind: 'node' as const, key }

  const previews = (group: FlowGroup): GroupPreview[] => group.members.flatMap((key) => {
    if (!key.startsWith('media:')) return []
    const media = mediaOf(canvas, key)
    return [{ key, title: mediaTitle(media), kind: media?.kind ?? 'text' }]
  })

  const placed = (node: FlowNode) => (live && live.key === node.key ? { ...node, x: live.x, y: live.y } : node)
  const byKey = new Map(nodes.map((n) => [n.key, placed(n)]))
  /** A node's title: its own name first, then the media description or the bot's name. */
  const titleOf = (key: string) => {
    const custom = stepNameOf(canvas, key)
    if (custom) return custom
    const media = mediaOf(canvas, key)
    if (media) return tr(mediaTitle(media))
    return bots.get(linkEndOf(key))?.name ?? tr('Unknown bot')
  }
  // An edge leaves exactly the ports and cards whose line is visible on the canvas.
  const connectedPorts = new Set(edges.map((edge) => edge.from))
  const search = query.trim().toLowerCase()
  const matches = (key: string, title: string) => !search || title.toLowerCase().includes(search) || key.toLowerCase().includes(search)
  const wireFrom = wire ? [...byKey.values()].find((n) => n.kind !== 'routine' && n.id === wire.from) : null
  const hovered = wire ? nodeAt([...byKey.values()], wire.x, wire.y) : null
  const zoom = (factor: number) => {
    const box = size()
    setView((v) => zoomAt(v, factor, box.width / 2, box.height / 2))
  }

  const edgeLabel = (edge: GroupedEdge, from: FlowNode, to: FlowNode) => {
    if (edge.count <= 1) return null
    const x = (from.x + NODE_WIDTH + to.x) / 2
    const y = (from.y + to.y) / 2 + NODE_HEIGHT / 2
    return <text className="flow-edge-count" x={x} y={y} aria-label={tr('{count} lines').replace('{count}', String(edge.count))}>×{edge.count}</text>
  }

  return (
    <div
      className="flow-viewport"
      ref={viewport}
      role="application"
      aria-label={tr('Routine flow canvas')}
      tabIndex={0}
      onPointerDown={startPan}
      onKeyDown={canvasKeys}
      onPaste={(event) => {
        const files = [...event.clipboardData.files]
        if (files.length >= 2) {
          event.preventDefault()
          onFiles(files.map((file) => ({ name: file.name })))
        }
      }}
      onDragOver={(event) => {
        if (event.dataTransfer.types.includes('Files')) event.preventDefault()
      }}
      onDrop={(event) => {
        const files = [...event.dataTransfer.files]
        if (files.length >= 2) {
          event.preventDefault()
          onFiles(files.map((file) => ({ name: file.name })))
        }
      }}
    >
      <div className="flow-layer" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` }}>
        <svg className="flow-edges" width="1" height="1" aria-hidden="true">
          {edges.map((edge) => {
            const from = byKey.get(edge.from)
            const to = byKey.get(edge.to)
            if (!from || !to) return null
            // A port is a small dot, not a card: its line leaves the dot, not a far-right card edge.
            const d = edge.from.startsWith(portPrefix)
              ? `M ${from.x + 10} ${from.y + 10} C ${from.x + 70} ${from.y + 10}, ${to.x - 50} ${to.y + NODE_HEIGHT / 2}, ${to.x} ${to.y + NODE_HEIGHT / 2}`
              : edgePath(from, to)
            const isSelected = selection?.kind === 'edge' && selection.id === edge.linkId
            return (
              <g key={edge.id}>
                <path className="flow-edge" data-kind={edge.linkId ? 'link' : 'start'} data-selected={isSelected} d={d} />
                {edge.linkId ? (
                  <path className="flow-edge-hit" d={d} onPointerDown={(e) => { e.stopPropagation(); onSelect({ kind: 'edge', id: edge.linkId! }) }} />
                ) : null}
                {edgeLabel(edge, from, to)}
              </g>
            )
          })}
          {wire && wireFrom ? (
            <path
              className="flow-edge"
              data-kind="wire"
              d={wire.from.startsWith(portPrefix)
                ? `M ${wireFrom.x + 10} ${wireFrom.y + 10} L ${wire.x} ${wire.y}`
                : `M ${wireFrom.x + NODE_WIDTH} ${wireFrom.y + NODE_HEIGHT / 2} L ${wire.x} ${wire.y}`}
            />
          ) : null}
        </svg>
        {nodes.map((raw) => {
          const node = placed(raw)
          // An output port: a small labelled dot a line can start from to reach every member.
          if (node.key.startsWith(portPrefix)) {
            const port = parsePort(node.key)!
            return (
              <button
                key={node.key}
                type="button"
                className="flow-port-node"
                data-carry={port.type === 'video' ? 'file' : port.type}
                data-connected={connectedPorts.has(node.key) ? 'true' : undefined}
                data-matched={matches(node.key, tr(PORT_LABEL[port.type] ?? port.type)) ? undefined : 'false'}
                style={{ transform: `translate(${node.x}px, ${node.y}px)` }}
                title={tr('Connect this output of the group')}
                aria-label={`${tr('Connect from this group')}: ${tr(PORT_LABEL[port.type] ?? port.type)}`}
                disabled={disabled}
                onPointerDown={(e) => { e.stopPropagation(); startWire(node.id, e) }}
              >
                <Icon name={PORT_ICON[port.type] ?? 'circle'} size={11} />
              </button>
            )
          }
          // A collapsed group's stack card.
          if (node.key.startsWith('group:')) {
            const group = groups.find((g) => g.id === node.id)
            if (!group) return null
            return (
              <FlowGroupCard
                key={raw.key}
                group={group}
                previews={previews(group)}
                x={node.x}
                y={node.y}
                selected={multi.has(node.key) || selection?.kind === 'group' && selection.id === group.id}
                disabled={disabled}
                onSelect={(e) => pick(node.key, e, group.id)}
                onDragStart={(e) => startGroupDrag(group, e)}
                onKey={(e) => keyOnNode(node, e)}
                onOpen={() => onToggleGroup(group.id)}
              />
            )
          }
          const card = raw.kind === 'routine' ? canvas.routines.find((c) => c.routine.id === raw.id) : undefined
          const media = raw.kind === 'media' ? mediaOf(canvas, raw.id) : null
          const step = raw.kind === 'routine' ? null : latestStep(canvas, raw.id)
          const stepCard = step ? canvas.routines.find((c) => c.steps.some((s) => s.id === step.id)) : undefined
          const working = bots.get(raw.id)
          const progress = stepCard && step && step.status === 'running' && working
            && (working.status === 'working' || working.status === 'needsInput')
            ? (() => {
              const p = stepProgress(stepCard, step)
              return { activity: working.activity, previous: p.previous ? titleOf(p.previous.botId) : '', done: p.done, total: p.total }
            })()
            : null
          const name = stepNameOf(canvas, raw.key) || (raw.kind === 'routine' ? card?.routine.name ?? '' : raw.kind === 'media' ? tr(mediaTitle(media)) : bots.get(raw.id)?.name ?? '')
          return (
            <FlowNodeCard
              key={raw.key}
              node={raw}
              x={node.x}
              y={node.y}
              bot={bots.get(raw.id)}
              routine={card}
              media={media}
              step={card ? stepOf(card, card.routine.botId) : step}
              unconnected={card ? isUnconnected(canvas, card.routine.id) : false}
              selected={multi.has(raw.key) || selection?.kind === 'node' && selection.key === raw.key}
              connectTarget={hovered?.key === raw.key && raw.kind !== 'routine' && raw.id !== wire?.from}
              disabled={disabled}
              onSelect={(e) => pick(raw.key, e)}
              onDragStart={(e) => startDrag(raw, e)}
              onPortStart={(e) => startWire(raw.id, e)}
              onKey={(e) => keyOnNode(raw, e)}
              onRun={raw.kind === 'routine' ? () => onRun(raw.id) : undefined}
              onStop={raw.kind === 'routine' ? () => onStop(raw.id) : undefined}
              onMenu={raw.kind === 'routine' ? undefined : (anchor) => setMenu({ key: raw.key, ...anchor })}
              stepName={stepNameOf(canvas, raw.key)}
              onRename={raw.kind === 'routine' || disabled ? undefined : (next) => onRenameStep(raw.key, next)}
              progress={progress}
              carry={portCarry(raw, media)}
              connected={connectedPorts.has(raw.key)}
              matched={matches(raw.key, name)}
            />
          )
        })}
        {selection?.kind === 'node' && (() => {
          const node = byKey.get(selection.key)
          const media = node?.key.startsWith('media:') ? mediaOf(canvas, node.id) : null
          if (!node || !media || disabled) return null
          return (
            <div
              className="flow-composer-anchor"
              style={{ transform: `translate(${node.x}px, ${node.y + NODE_HEIGHT + 12}px)` }}
              onPointerDown={(event) => event.stopPropagation()}
            >
              <MediaComposer
                media={media}
                canvas={canvas}
                busy={composer.busy}
                offline={composer.offline}
                targets={composer.targets}
                onSaveMedia={composer.onSaveMedia}
                onRunFromStep={composer.onRunFromStep}
                onClose={() => onSelect(null)}
              />
            </div>
          )
        })()}
      </div>
      {menu && menuPresence.mounted ? (() => {
        const rows = menuItems(menu.key)
        if (rows.length === 0) return null
        const rect = size()
        const at = flowMenuPoint(
          { x: menu.x - rect.left, top: menu.top - rect.top, bottom: menu.bottom - rect.top },
          box,
          { width: 220, height: rows.length * 34 + 12 },
        )
        const maxHeight = Math.max(120, box.height - at.y - 8)
        return (
          <div
            ref={menuLayer}
            className={`flow-node-menu ${menuPresence.shown ? 'shown' : ''}`}
            style={{ left: at.x, top: at.y, maxWidth: 220, maxHeight }}
            onPointerDown={(event) => event.stopPropagation()}
          >
            <MenuPanel items={rows} dismiss={() => setMenu(null)} maxHeight={maxHeight} />
          </div>
        )
      })() : null}
      <MiniMap
        nodes={nodes.map(placed)}
        view={view}
        box={box}
        onCenter={(x, y) => setView((v) => ({ ...v, x: box.width / 2 - x * v.scale, y: box.height / 2 - y * v.scale }))}
      />
      <div className="flow-toolbar">
        <IconButton title={tr('Zoom out')} icon="minus" onClick={() => zoom(1 / 1.25)} />
        <span className="flow-zoom-value" aria-label={tr('Zoom level')}>{Math.round(view.scale * 100)}%</span>
        <IconButton title={tr('Zoom in')} icon="plus" onClick={() => zoom(1.25)} />
        <span className="flow-toolbar-separator" aria-hidden />
        <IconButton title={tr('Fit to view')} icon="arrow.up.left.and.arrow.down.right" onClick={() => setView(fitView(nodes, box.width, box.height))} />
        <IconButton title={tr('Tidy up')} icon="square.grid.2x2" onClick={onTidy} />
        {onUndoTidy ? <IconButton title={tr('Undo tidy up')} icon="arrow.uturn.backward" onClick={onUndoTidy} /> : null}
      </div>
    </div>
  )
}
