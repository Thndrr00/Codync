import { useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import type { DiagramElement, DiagramScene, DiagramTool } from '@shared/diagrams'
import {
  DEFAULT_FILL, DEFAULT_STROKE, DIAGRAM_TOOLS, SWATCHES, addElement, findElement, newElement, parseScene,
  removeElement, sceneBounds, serializeScene, updateElement,
} from '@shared/diagrams'
import { Icon } from '../../components/Icon'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import type { Artifact } from '@shared/artifacts'
import './diagram.css'

/** An editable Excalidraw-shaped scene, drawn as SVG: no bundled Excalidraw app. */
export function DiagramView({ botId, artifact, source, onError }: {
  botId: string
  artifact: Artifact
  source: string
  onError: (message: string | null) => void
}) {
  const store = useStore()
  const tr = useTr()
  const [scene, setScene] = useState<DiagramScene | null>(() => {
    try {
      return parseScene(source)
    } catch {
      return null
    }
  })
  const [selected, setSelected] = useState<string | null>(null)
  const [tool, setTool] = useState<DiagramTool>('select')
  const [saved, setSaved] = useState(() => {
    try {
      return serializeScene(parseScene(source))
    } catch {
      return ''
    }
  })
  const [status, setStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const svg = useRef<SVGSVGElement | null>(null)
  const drag = useRef<{ id: string; x: number; y: number; originX: number; originY: number } | null>(null)
  const bounds = useMemo(() => scene ? sceneBounds(scene) : { x: 0, y: 0, width: 640, height: 400 }, [scene])
  if (!scene) {
    return <div className="artifact-state"><VStack gap={6}>
      <Text weight="semibold">{tr('This diagram could not be opened')}</Text>
      <Text size="caption" tone="secondary">{tr('Its scene is not valid Excalidraw JSON.')}</Text>
    </VStack></div>
  }
  const dirty = serializeScene(scene) !== saved
  const selectedElement = findElement(scene, selected)
  const change = (next: DiagramScene) => { setScene(next); setStatus('idle'); onError(null) }

  const point = (event: ReactPointerEvent) => {
    const node = svg.current
    if (!node) return { x: 0, y: 0 }
    const matrix = node.getScreenCTM()?.inverse()
    if (!matrix) return { x: 0, y: 0 }
    const p = node.createSVGPoint()
    p.x = event.clientX
    p.y = event.clientY
    const local = p.matrixTransform(matrix)
    return { x: local.x, y: local.y }
  }

  const down = (event: ReactPointerEvent) => {
    const target = (event.target as Element).closest('[data-element-id]')
    if (tool !== 'select') {
      const at = point(event)
      const element = newElement(tool, at.x, at.y, tool === 'text' ? 'Text' : '')
      change(addElement(scene, element))
      setSelected(element.id)
      setTool('select')
      return
    }
    const id = target?.getAttribute('data-element-id') ?? null
    setSelected(id)
    if (!id) return
    const element = findElement(scene, id)
    if (!element) return
    const at = point(event)
    drag.current = { id, x: at.x, y: at.y, originX: element.x, originY: element.y }
    ;(event.currentTarget as SVGSVGElement).setPointerCapture(event.pointerId)
  }

  const move = (event: ReactPointerEvent) => {
    const current = drag.current
    if (!current) return
    const at = point(event)
    change(updateElement(scene, current.id, {
      x: Math.round(current.originX + at.x - current.x),
      y: Math.round(current.originY + at.y - current.y),
    }))
  }

  const save = async () => {
    setStatus('saving')
    onError(null)
    try {
      const current = store.artifacts.byId.get(artifact.id) ?? artifact
      await store.artifacts.saveScene(store.client, botId, artifact.id, scene, current.sha256)
      setSaved(serializeScene(scene))
      setStatus('saved')
    } catch (error) {
      setStatus('error')
      onError(error instanceof Error ? error.message : String(error))
    }
  }

  return <div className="diagram-view">
    <div className="diagram-tools" role="toolbar" aria-label={tr('Diagram tools')}>
      {DIAGRAM_TOOLS.map((choice) => <button key={choice.id} className={`diagram-tool ${tool === choice.id ? 'active' : ''}`}
        aria-pressed={tool === choice.id} title={tr(choice.label)} onClick={() => setTool(choice.id)}>
        <Icon name={choice.icon} size={15} /><span>{tr(choice.label)}</span>
      </button>)}
      <span className="diagram-spacer" />
      <Text as="span" size="caption" tone="secondary">{scene.elements.length === 1
        ? tr('1 element')
        : tr('{n} elements').replaceAll('{n}', () => String(scene.elements.length))}</Text>
      <button className="diagram-save press" disabled={!dirty || status === 'saving'} onClick={() => void save()}>
        {status === 'saving' ? tr('Saving…') : tr('Save diagram')}
      </button>
    </div>
    <div className="diagram-body">
      <svg ref={svg} className={`diagram-canvas tool-${tool}`} viewBox={`${bounds.x} ${bounds.y} ${bounds.width} ${bounds.height}`}
        role="img" aria-label={tr('Editable diagram')} onPointerDown={down} onPointerMove={move} onPointerUp={() => { drag.current = null }}>
        <defs>
          <marker id="diagram-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M 0 0 L 10 5 L 0 10 z" fill={DEFAULT_STROKE} />
          </marker>
        </defs>
        {scene.elements.map((element) => <Element key={element.id} element={element} selected={selected === element.id} />)}
      </svg>
      <aside className="diagram-inspector" aria-label={tr('Diagram element')}>
        {selectedElement ? <VStack gap={10}>
          <Text weight="semibold">{tr(DIAGRAM_TOOLS.find((choice) => choice.id === selectedElement.type)?.label ?? 'Diagram element')}</Text>
          {(selectedElement.type === 'text' || selectedElement.text !== undefined || ['rectangle', 'ellipse', 'diamond'].includes(selectedElement.type)) && <label className="diagram-field">
            <span>{tr('Label')}</span>
            <input value={selectedElement.text ?? ''} onChange={(event) => change(updateElement(scene, selectedElement.id, { text: event.target.value }))} />
          </label>}
          <div className="diagram-field"><span>{tr('Stroke')}</span><div className="diagram-swatches">
            {SWATCHES.map((color) => <button key={color} className={`diagram-swatch ${(selectedElement.strokeColor ?? DEFAULT_STROKE) === color ? 'active' : ''}`}
              style={{ background: color }} title={color} aria-label={`${tr('Stroke')} ${color}`} onClick={() => change(updateElement(scene, selectedElement.id, { strokeColor: color }))} />)}
          </div></div>
          {['rectangle', 'ellipse', 'diamond'].includes(selectedElement.type) && <div className="diagram-field"><span>{tr('Fill')}</span><div className="diagram-swatches">
            <button className={`diagram-swatch none ${(selectedElement.backgroundColor ?? DEFAULT_FILL) === DEFAULT_FILL ? 'active' : ''}`}
              title={tr('No fill')} aria-label={tr('No fill')} onClick={() => change(updateElement(scene, selectedElement.id, { backgroundColor: DEFAULT_FILL }))} />
            {SWATCHES.map((color) => <button key={color} className={`diagram-swatch ${selectedElement.backgroundColor === color ? 'active' : ''}`}
              style={{ background: color }} title={color} aria-label={`${tr('Fill')} ${color}`} onClick={() => change(updateElement(scene, selectedElement.id, { backgroundColor: color }))} />)}
          </div></div>}
          <button className="diagram-delete press" onClick={() => { change(removeElement(scene, selectedElement.id)); setSelected(null) }}>
            <Icon name="trash" size={14} /> {tr('Delete element')}
          </button>
        </VStack> : <VStack gap={6}>
          <Text size="caption" tone="secondary">{tr('Pick a shape, or select an element to change its text and colors.')}</Text>
        </VStack>}
      </aside>
    </div>
    <Text as="div" size="caption" tone={status === 'error' ? 'danger' : 'secondary'} aria-live="polite">
      {status === 'saving' ? tr('Saving…') : dirty ? tr('Unsaved changes · this diagram saves when you choose Save diagram.') : tr('All changes saved.')}
    </Text>
  </div>
}

function Element({ element, selected }: { element: DiagramElement; selected: boolean }) {
  const stroke = element.strokeColor ?? DEFAULT_STROKE
  const fill = element.backgroundColor ?? DEFAULT_FILL
  const width = Math.max(1, element.width)
  const height = Math.max(1, element.height)
  const text = element.text ?? ''
  const common = { 'data-element-id': element.id, className: selected ? 'diagram-element selected' : 'diagram-element' }
  if (element.type === 'ellipse') {
    return <g {...common}>
      <ellipse cx={element.x + width / 2} cy={element.y + height / 2} rx={width / 2} ry={height / 2} fill={fill} stroke={stroke} strokeWidth={element.strokeWidth ?? 2} />
      <Label element={element} text={text} />
    </g>
  }
  if (element.type === 'diamond') {
    const points = `${element.x + width / 2},${element.y} ${element.x + width},${element.y + height / 2} ${element.x + width / 2},${element.y + height} ${element.x},${element.y + height / 2}`
    return <g {...common}>
      <polygon points={points} fill={fill} stroke={stroke} strokeWidth={element.strokeWidth ?? 2} />
      <Label element={element} text={text} />
    </g>
  }
  if (element.type === 'arrow' || element.type === 'line') {
    const points = (element.points?.length ? element.points : [[0, 0], [width, 0]]).map(([x, y]) => `${element.x + x},${element.y + y}`).join(' ')
    return <g {...common}>
      <polyline points={points} fill="none" stroke={stroke} strokeWidth={element.strokeWidth ?? 2}
        markerEnd={element.type === 'arrow' ? 'url(#diagram-arrow)' : undefined} />
    </g>
  }
  if (element.type === 'text') {
    return <g {...common}>
      <text x={element.x} y={element.y + (element.fontSize ?? 20)} fill={stroke} fontSize={element.fontSize ?? 20}>{text || 'Text'}</text>
    </g>
  }
  if (element.type === 'freedraw') {
    const points = (element.points ?? []).map(([x, y]) => `${element.x + x},${element.y + y}`).join(' ')
    return <g {...common}><polyline points={points} fill="none" stroke={stroke} strokeWidth={element.strokeWidth ?? 2} /></g>
  }
  return <g {...common}>
    <rect x={element.x} y={element.y} width={width} height={height} rx={8} fill={fill} stroke={stroke} strokeWidth={element.strokeWidth ?? 2} />
    <Label element={element} text={text} />
  </g>
}

function Label({ element, text }: { element: DiagramElement; text: string }) {
  if (!text) return null
  return <text x={element.x + element.width / 2} y={element.y + element.height / 2} textAnchor="middle" dominantBaseline="middle"
    fill={element.strokeColor ?? DEFAULT_STROKE} fontSize={element.fontSize ?? 18}>{text}</text>
}
