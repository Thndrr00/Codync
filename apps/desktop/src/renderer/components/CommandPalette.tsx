import { useEffect, useMemo, useRef, useState } from 'react'
import { useTr } from '../lib/use-language'
import { Icon } from './Icon'
import { Sheet } from './Overlay'
import { HStack } from './Stack'
import { Text } from './Text'
import { shortcutKeys } from './keyboard-shortcuts'
import './command-palette.css'

/** One pick in the palette. `group` is the heading it sits under; `keywords` only helps the filter. */
export interface Command {
  id: string
  title: string
  group: string
  icon?: string
  keywords?: string
  shortcutId?: string
  run: () => void
}

/**
 * Subsequence match with bonuses for consecutive letters and word starts; a negative score is no
 * match. Enough to find "setg" in "Settings" without a fuzzy-matching dependency.
 */
export function fuzzyScore(text: string, query: string): number {
  const q = query.trim().toLocaleLowerCase()
  if (!q) return 0
  const value = text.toLocaleLowerCase()
  let score = 0
  let at = 0
  let streak = 0
  for (const letter of q) {
    const found = value.indexOf(letter, at)
    if (found < 0) return -1
    streak = found === at ? streak + 1 : 0
    const wordStart = found === 0 || /[\s\-_/]/.test(value[found - 1] ?? '')
    score += 1 + streak * 2 + (wordStart ? 4 : 0)
    at = found + 1
  }
  return score - (value.length - q.length) * 0.05
}

/** The shared command overlay: grouped commands, a fuzzy filter and ↑/↓/Enter navigation. */
export function CommandPalette({ open, onClose, commands }: { open: boolean; onClose: () => void; commands: Command[] }) {
  return (
    <Sheet open={open} onClose={onClose} width={640} dismissOnDim>
      {open ? <PaletteBody commands={commands} onClose={onClose} /> : null}
    </Sheet>
  )
}

function PaletteBody({ commands, onClose }: { commands: Command[]; onClose: () => void }) {
  const tr = useTr()
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const list = useRef<HTMLDivElement>(null)

  // Keep the caller's group order, sort within a group by how well it matches.
  const groups = useMemo(() => {
    const names = [...new Set(commands.map((command) => command.group))]
    return names.map((name) => ({
      name,
      items: commands
        .filter((command) => command.group === name)
        .map((command) => ({ command, score: Math.max(fuzzyScore(command.title, query), query ? fuzzyScore(command.keywords ?? '', query) : -1) }))
        .filter((hit) => hit.score >= 0)
        .sort((a, b) => b.score - a.score)
        .map((hit) => hit.command),
    })).filter((group) => group.items.length > 0)
  }, [commands, query])
  const flat = groups.flatMap((group) => group.items)
  const indexOf = new Map(flat.map((command, index) => [command, index]))
  // Bots can come and go while the palette is open; keep the highlight on a real row.
  const current = Math.min(active, Math.max(0, flat.length - 1))

  useEffect(() => setActive(0), [query])
  useEffect(() => {
    list.current?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' })
  }, [active, groups])

  // Escape is handled by Sheet's useEscape, so it closes the newest layer even with a menu on top.
  const onKey = (event: React.KeyboardEvent) => {
    if (event.nativeEvent.isComposing) return
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const step = event.key === 'ArrowDown' ? 1 : -1
      setActive((current) => (flat.length ? (current + step + flat.length) % flat.length : 0))
    } else if (event.key === 'Enter') {
      event.preventDefault()
      const command = flat[current]
      if (command) {
        onClose()
        command.run()
      }
    }
  }

  return (
    <div className="command-palette" onKeyDown={onKey}>
      <HStack gap={10} className="command-palette-field">
        <Icon name="magnifyingglass" size={16} />
        <input
          data-autofocus
          autoFocus
          value={query}
          placeholder={tr('Search bots, settings and actions')}
          aria-label={tr('Search commands')}
          spellCheck={false}
          onChange={(event) => setQuery(event.target.value)}
        />
        <span className="command-key" aria-hidden>Esc</span>
      </HStack>
      <div className="command-palette-list" ref={list} role="listbox" aria-label={tr('Commands')}>
        {groups.map((group) => (
          <div key={group.name} className="command-group" role="group" aria-label={tr(group.name)}>
            <Text as="div" size="caption" weight="semibold" tone="secondary" className="command-group-title">{tr(group.name)}</Text>
            {group.items.map((command) => {
              const selected = indexOf.get(command) === current
              return (
                <button key={command.id} type="button" role="option" aria-selected={selected}
                  className={`command-row ${selected ? 'active' : ''}`}
                  onMouseMove={() => setActive(indexOf.get(command) ?? 0)}
                  onClick={() => { onClose(); command.run() }}>
                  <Icon name={command.icon ?? 'bolt'} size={16} />
                  <Text grow truncate>{tr(command.title)}</Text>
                  {command.shortcutId && shortcutKeys(command.shortcutId) ? (
                    <span className="command-key" aria-hidden>{shortcutKeys(command.shortcutId)}</span>
                  ) : null}
                </button>
              )
            })}
          </div>
        ))}
        {!flat.length ? <Text as="div" tone="secondary" className="command-empty">{tr('No matching commands')}</Text> : null}
      </div>
      <div className="command-palette-footer" aria-hidden>
        <span className="command-key">↑</span><span className="command-key">↓</span>{tr('navigate')}
        <span className="command-key">↵</span>{tr('select')}
      </div>
    </div>
  )
}
