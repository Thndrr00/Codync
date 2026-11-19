import { useEffect, useRef, type CSSProperties, type KeyboardEvent, type PointerEvent } from 'react'
import { Icon } from '../../../components/Icon'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { reduceMotion } from '../../../lib/theme'
import { useTr } from '../../../lib/use-language'
import { NODE_HEIGHT, NODE_WIDTH } from '../../../lib/routine-flow.ts'
import type { FlowGroup } from './flow-groups'

/** A picture preview on a collapsed group's stack card. */
export interface GroupPreview {
  key: string
  title: string
  kind: 'image' | 'video' | 'text'
}

interface Props {
  group: FlowGroup
  previews: GroupPreview[]
  x: number
  y: number
  selected: boolean
  disabled: boolean
  onSelect: (event: PointerEvent) => void
  onDragStart: (event: PointerEvent) => void
  onKey: (event: KeyboardEvent) => void
  /** Double-click opens the group again. */
  onOpen: () => void
}

/**
 * A group folded into a stack card: its name and a preview of the pictures it holds. Its output
 * ports are drawn as their own small nodes beside it, so a line from one port reaches every member.
 * Appearing (a group just collapsed) plays the same quiet scale-in the rest of the app uses; Motion
 * is the Web Animations API here, skipped when the person prefers reduced motion.
 */
export function FlowGroupCard({ group, previews, x, y, selected, disabled, onSelect, onDragStart, onKey, onOpen }: Props) {
  const tr = useTr()
  const card = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (reduceMotion() || !card.current) return
    card.current.animate(
      [{ opacity: 0, transform: `translate(${x}px, ${y}px) scale(0.92)` }, { opacity: 1, transform: `translate(${x}px, ${y}px) scale(1)` }],
      { duration: 240, easing: 'cubic-bezier(0.2, 0.9, 0.3, 1)' },
    )
    // Animate only when the card appears (a group was collapsed), not on every drag frame.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
  return (
    <div
      ref={card}
      className="flow-node flow-group-card"
      data-kind="group"
      data-selected={selected}
      role="button"
      tabIndex={0}
      aria-label={`${tr('Group')}: ${group.name || tr('Untitled group')}`}
      aria-pressed={selected}
      style={{ transform: `translate(${x}px, ${y}px)`, width: NODE_WIDTH, minHeight: NODE_HEIGHT } as CSSProperties}
      onPointerDown={(event) => {
        onSelect(event)
        if (!disabled) onDragStart(event)
      }}
      onDoubleClick={(event) => {
        event.stopPropagation()
        onOpen()
      }}
      onKeyDown={onKey}
    >
      <HStack gap={8} align="center">
        <Icon name="photo.on.rectangle" size={22} />
        <VStack gap={1} shrink>
          <Text weight="semibold" truncate>{group.name || tr('Untitled group')}</Text>
          <Text size="caption" tone="secondary">{tr('{count} nodes').replace('{count}', String(group.members.length))}</Text>
        </VStack>
      </HStack>
      {previews.length > 0 ? (
        <HStack gap={4} className="flow-group-previews" aria-label={tr('Pictures in this group')}>
          {previews.slice(0, 4).map((preview) => (
            <span key={preview.key} className="flow-group-preview" title={preview.title}>
              <Icon name={preview.kind === 'video' ? 'film' : preview.kind === 'image' ? 'photo' : 'bubble.left'} size={16} />
            </span>
          ))}
          {previews.length > 4 ? <Text size="caption" tone="secondary">+{previews.length - 4}</Text> : null}
        </HStack>
      ) : null}
      <Text size="caption" tone="secondary">{tr('Double-click to open')}</Text>
    </div>
  )
}
