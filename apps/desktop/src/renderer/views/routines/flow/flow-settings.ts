// What a routine-flow step actually ran with, sent beside the steps on the canvas. A run saved
// before steps kept settings has no entry: the canvas shows the result and no settings, not an error.
// The shape mirrors `engine::StepSettings` in host/src/routines/flow/engine.rs and lives in
// lib/routine-flow.ts, so the canvas can read a batch's progress beside its steps.

import { NODE_HEIGHT, NODE_WIDTH, type FlowNode, type FlowStepInput, type FlowStepSettings, type WorkflowNodeSize } from '../../../lib/routine-flow.ts'

export type { FlowStepInput, FlowStepSettings }

export type StepSettingsMap = Record<string, FlowStepSettings>

/** The settings a step ran with, or null for an older run that saved none. */
export function settingsOf(map: StepSettingsMap | undefined, stepId: string): FlowStepSettings | null {
    return map?.[stepId] ?? null
}

const PREVIEW_CHARS = 180

/** A long prompt or input shows a few lines; "Show more" reveals the rest. */
export function clipped(text: string, expanded: boolean): string {
    if (expanded || text.length <= PREVIEW_CHARS) return text
    return `${text.slice(0, PREVIEW_CHARS).trimEnd()}…`
}

export const hasMore = (text: string): boolean => text.length > PREVIEW_CHARS

/** A node menu opens below its button when there is room, else above, always inside the canvas. */
export function flowMenuPoint(
    anchor: { x: number; top: number; bottom: number },
    box: { width: number; height: number },
    panel: { width: number; height: number },
): { x: number; y: number } {
    const x = Math.min(Math.max(8, anchor.x), Math.max(8, box.width - panel.width - 8))
    const roomBelow = box.height - anchor.bottom - 8
    const y = roomBelow >= panel.height
        ? Math.min(anchor.bottom + 6, Math.max(8, box.height - panel.height - 8))
        : Math.max(8, anchor.top - panel.height - 6)
    return { x, y }
}

/** The leftmost node, or the topmost one when several share a column. Stable when positions tie. */
export function leftmostNode(nodes: readonly FlowNode[]): FlowNode | null {
    return nodes.reduce<FlowNode | null>((best, node) => {
        if (!best) return node
        if (node.x < best.x - 1 || (Math.abs(node.x - best.x) <= 1 && node.y < best.y)) return node
        return best
    }, null)
}

const NODE_SIZE_MIN = 120
const NODE_SIZE_MAX = 900
const FONT_MIN = 11
const FONT_MAX = 36

/** Clamps a resized node to a legible box and font, keeping the values the person didn't change. */
export function clampNodeSize(size: WorkflowNodeSize): WorkflowNodeSize {
    const bound = (value: number | undefined, low: number, high: number, fallback: number) =>
        typeof value === 'number' && Number.isFinite(value) ? Math.round(Math.min(high, Math.max(low, value))) : fallback
    return {
        width: bound(size.width, NODE_SIZE_MIN, NODE_SIZE_MAX, NODE_WIDTH),
        height: bound(size.height, NODE_SIZE_MIN, NODE_SIZE_MAX, NODE_HEIGHT),
        font: size.font === undefined ? undefined : bound(size.font, FONT_MIN, FONT_MAX, 14),
    }
}

/** Thins a raw pointer path to points at least `gap` pixels apart, so a saved stroke stays small. */
export function sampleStroke<T extends { x: number; y: number }>(points: readonly T[], gap = 2): T[] {
    if (points.length === 0) return []
    const kept: T[] = [points[0]]
    for (const point of points.slice(1)) {
        const last = kept[kept.length - 1]
        if (Math.hypot(point.x - last.x, point.y - last.y) >= gap) kept.push(point)
    }
    return kept
}
