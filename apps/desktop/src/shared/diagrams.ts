// Diagram scenes in the Excalidraw JSON shape, drawn by Codync itself as SVG (no Excalidraw React app).
// The host validates and stores the same shape (`host/src/chat/diagrams.rs`).
export interface DiagramElement {
  id: string
  type: string
  x: number
  y: number
  width: number
  height: number
  angle?: number
  strokeColor?: string
  backgroundColor?: string
  strokeWidth?: number
  strokeStyle?: 'solid' | 'dashed' | 'dotted'
  opacity?: number
  text?: string
  fontSize?: number
  fontFamily?: number
  textAlign?: 'left' | 'center' | 'right'
  verticalAlign?: 'top' | 'middle' | 'bottom'
  points?: [number, number][]
  version?: number
  versionNonce?: number
  seed?: number
  isDeleted?: boolean
  groupIds?: string[]
  roundness?: unknown
  roughness?: number
  [key: string]: unknown
}

export interface DiagramScene {
  type: 'excalidraw'
  version: number
  source?: string
  elements: DiagramElement[]
  appState?: Record<string, unknown>
  files?: Record<string, unknown>
}

export type DiagramTool = 'select' | 'rectangle' | 'ellipse' | 'diamond' | 'arrow' | 'text'

export const DIAGRAM_TOOLS: { id: DiagramTool; label: string; icon: string }[] = [
  { id: 'select', label: 'Select', icon: 'cursorarrow' },
  { id: 'rectangle', label: 'Rectangle', icon: 'rectangle' },
  { id: 'ellipse', label: 'Ellipse', icon: 'circle' },
  { id: 'diamond', label: 'Diamond', icon: 'diamond' },
  { id: 'arrow', label: 'Arrow', icon: 'arrow.right' },
  { id: 'text', label: 'Text', icon: 'textformat' },
]

export const DEFAULT_STROKE = '#1e1e1e'
export const DEFAULT_FILL = 'transparent'
export const SWATCHES = ['#1e1e1e', '#e03131', '#2f9e44', '#1971c2', '#7048e8', '#f08c00']

const nonce = () => Math.floor(Math.random() * 2_147_483_647)

export function emptyScene(): DiagramScene {
  return {
    type: 'excalidraw',
    version: 2,
    source: 'codync',
    elements: [],
    appState: { viewBackgroundColor: '#ffffff', gridSize: 20 },
  }
}

export function newElement(tool: Exclude<DiagramTool, 'select'>, x: number, y: number, text = ''): DiagramElement {
  const base: DiagramElement = {
    id: crypto.randomUUID(),
    type: tool,
    x: Math.round(x),
    y: Math.round(y),
    width: 160,
    height: 90,
    angle: 0,
    strokeColor: DEFAULT_STROKE,
    backgroundColor: DEFAULT_FILL,
    strokeWidth: 2,
    strokeStyle: 'solid',
    opacity: 100,
    seed: nonce(),
    version: 1,
    versionNonce: nonce(),
    groupIds: [],
  }
  if (tool === 'text') return { ...base, width: 220, height: 36, text, fontSize: 20, fontFamily: 1, textAlign: 'left', verticalAlign: 'top' }
  if (tool === 'arrow') return { ...base, width: 0, height: 0, points: [[0, 0], [160, 0]] }
  return base
}

/** A scene from stored JSON; anything that is not one is an error the viewer shows. */
export function parseScene(source: string): DiagramScene {
  const value: unknown = JSON.parse(source)
  if (!value || typeof value !== 'object') throw new Error('This diagram is not a scene')
  const scene = value as Partial<DiagramScene>
  if (scene.type !== 'excalidraw' || !Array.isArray(scene.elements)) throw new Error('This diagram is not an Excalidraw scene')
  const elements = scene.elements.filter((element): element is DiagramElement => {
    return !!element && typeof element === 'object' && typeof element.id === 'string' && typeof element.type === 'string'
      && Number.isFinite(element.x) && Number.isFinite(element.y)
  })
  return {
    type: 'excalidraw',
    version: typeof scene.version === 'number' ? scene.version : 2,
    source: typeof scene.source === 'string' ? scene.source : 'codync',
    elements: elements.map((element) => ({ width: 0, height: 0, ...element })),
    appState: scene.appState && typeof scene.appState === 'object' ? scene.appState : {},
  }
}

/** Whether an artifact is a diagram the viewer edits instead of a page it frames. */
export function isDiagramKind(kind: string | undefined): boolean {
  return kind === 'diagram'
}

export function serializeScene(scene: DiagramScene): string {
  return JSON.stringify(scene)
}

export function addElement(scene: DiagramScene, element: DiagramElement): DiagramScene {
  return { ...scene, elements: [...scene.elements, element] }
}

export function updateElement(scene: DiagramScene, id: string, patch: Partial<DiagramElement>): DiagramScene {
  return { ...scene, elements: scene.elements.map((element) => element.id === id ? { ...element, ...patch } : element) }
}

export function removeElement(scene: DiagramScene, id: string): DiagramScene {
  return { ...scene, elements: scene.elements.filter((element) => element.id !== id) }
}

export function findElement(scene: DiagramScene, id: string | null): DiagramElement | undefined {
  return id ? scene.elements.find((element) => element.id === id) : undefined
}

/** The drawing's extent, with room around it for selection handles. */
export function sceneBounds(scene: DiagramScene): { x: number; y: number; width: number; height: number } {
  if (!scene.elements.length) return { x: 0, y: 0, width: 640, height: 400 }
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const element of scene.elements) {
    const points = element.type === 'arrow' || element.type === 'line' ? element.points ?? [] : []
    const xs = [element.x, element.x + element.width, ...points.map(([x]) => element.x + x)]
    const ys = [element.y, element.y + element.height, ...points.map(([, y]) => element.y + y)]
    minX = Math.min(minX, ...xs); minY = Math.min(minY, ...ys)
    maxX = Math.max(maxX, ...xs); maxY = Math.max(maxY, ...ys)
  }
  const pad = 48
  return { x: minX - pad, y: minY - pad, width: Math.max(320, maxX - minX + pad * 2), height: Math.max(200, maxY - minY + pad * 2) }
}

/** A stable, readable line for the scene's accessible description. */
export function sceneSummary(scene: DiagramScene): string {
  const count = scene.elements.length
  if (!count) return 'No diagram elements yet.'
  const labels = scene.elements.map((element) => element.text?.trim() || element.type).filter(Boolean)
  return `${count} element${count === 1 ? '' : 's'}: ${labels.slice(0, 4).join(', ')}${labels.length > 4 ? '…' : ''}`
}
