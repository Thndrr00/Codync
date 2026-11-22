import { useEffect, useRef, useState, type MouseEvent } from 'react'
import { AnchoredMenu, type MenuItem } from '../../../components/Menu'
import { TabStrip, type StripTab } from '../../../components/TabStrip'
import { useTr } from '../../../lib/use-language'

/**
 * The flow view's title-bar tabs. TabStrip supplies the row, drag reorder, overflow and middle-click
 * close; this adds the two OpenBot title-bar gestures the shared strip deliberately doesn't own:
 * a double-click renames the workflow inline, and a per-tab menu closes it, its siblings or all
 * tabs to its right.
 */
export function FlowTabStrip({ tabs, active, nameOf, running, onSelect, onClose, onCloseOthers, onCloseRight, onReorder, onRename }: {
  tabs: readonly string[]
  active: string
  nameOf: (id: string) => string
  running: (id: string) => boolean
  onSelect: (id: string) => void
  onClose: (id: string) => void
  onCloseOthers: (id: string) => void
  onCloseRight: (id: string) => void
  onReorder: (from: string, to: string) => void
  onRename: (id: string, name: string) => void
}) {
  const tr = useTr()
  const [renaming, setRenaming] = useState<{ id: string; rect: DOMRect } | null>(null)
  const [menu, setMenu] = useState<{ id: string; x: number; y: number } | null>(null)
  const items: StripTab<string>[] = tabs.map((id) => ({
    id,
    title: nameOf(id),
    icon: running(id) ? 'bolt' : undefined,
  }))

  const tabAt = (event: MouseEvent<HTMLDivElement>): { id: string; element: HTMLElement } | null => {
    const element = (event.target as HTMLElement).closest<HTMLElement>('.tab-strip-tab')
    const id = element?.dataset.id
    return id ? { id, element } : null
  }

  const startRename = (event: MouseEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest('.tab-strip-close')) return
    const found = tabAt(event)
    if (!found) return
    event.preventDefault()
    event.stopPropagation()
    setRenaming({ id: found.id, rect: found.element.getBoundingClientRect() })
  }

  const openMenu = (event: MouseEvent<HTMLDivElement>) => {
    const found = tabAt(event)
    if (!found) return
    event.preventDefault()
    setMenu({ id: found.id, x: event.clientX, y: event.clientY })
  }

  const menuItems = (): MenuItem[] => {
    if (!menu) return []
    const single = tabs.length < 2
    return [
      { title: 'Close tab', icon: 'xmark', action: () => onClose(menu.id) },
      { title: 'Close other tabs', icon: 'xmark.circle', disabled: single, action: () => onCloseOthers(menu.id) },
      { title: 'Close tabs to the right', icon: 'chevron.right', disabled: single, action: () => onCloseRight(menu.id) },
    ]
  }

  return (
    <>
      <div className="flow-tab-strip" onDoubleClick={startRename} onContextMenu={openMenu}>
        <TabStrip
          tabs={items}
          active={active}
          label="Workflow tabs"
          onSelect={onSelect}
          onClose={onClose}
          onReorder={onReorder}
        />
      </div>
      {renaming ? (
        <TabNameEditor
          rect={renaming.rect}
          name={nameOf(renaming.id)}
          label={tr('Workflow name')}
          onDone={(name) => {
            if (name !== null) onRename(renaming.id, name)
            setRenaming(null)
          }}
        />
      ) : null}
      <AnchoredMenu open={menu !== null} onClose={() => setMenu(null)} point={menu ? { x: menu.x, y: menu.y } : null} items={menuItems} />
    </>
  )
}

/** The shared inline-edit pattern (RosterNameEditor), positioned over the tab being renamed. */
function TabNameEditor({ rect, name, label, onDone }: { rect: DOMRect; name: string; label: string; onDone: (name: string | null) => void }) {
  const [value, setValue] = useState(name)
  const field = useRef<HTMLInputElement>(null)
  const saving = useRef(false)
  useEffect(() => {
    field.current?.focus()
    field.current?.select()
  }, [])
  const save = () => {
    if (saving.current) return
    saving.current = true
    const next = value.trim()
    onDone(next && next !== name ? next : null)
  }
  return (
    <input
      ref={field}
      className="flow-tab-rename"
      style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height }}
      value={value}
      aria-label={label}
      spellCheck={false}
      maxLength={200}
      onClick={(event) => event.stopPropagation()}
      onDoubleClick={(event) => event.stopPropagation()}
      onPointerDownCapture={(event) => event.stopPropagation()}
      onChange={(event) => setValue(event.target.value)}
      onBlur={save}
      onKeyDown={(event) => {
        event.stopPropagation()
        if (event.nativeEvent.isComposing) return
        if (event.key === 'Enter') { event.preventDefault(); save() }
        else if (event.key === 'Escape') { event.preventDefault(); onDone(null) }
      }}
    />
  )
}
