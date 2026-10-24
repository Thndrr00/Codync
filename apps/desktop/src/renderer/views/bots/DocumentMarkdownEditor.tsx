import { useLayoutEffect, useRef, useState } from 'react'
import { documentSlash } from '../../lib/document-blocks'
import type { DOCUMENT_BLOCKS } from '../../lib/document-blocks'
import { Button } from '../../components/Controls'

type Block = typeof DOCUMENT_BLOCKS[number]

/** Plain Markdown with an inline, keyboard-accessible snippet chooser. */
export function DocumentMarkdownEditor({ value, disabled, onChange }: {
  value: string; disabled: boolean; onChange: (value: string) => void
}) {
  const input = useRef<HTMLTextAreaElement>(null)
  const restore = useRef<number | null>(null)
  const [cursor, setCursor] = useState(-1)
  const [selected, setSelected] = useState(0)
  const [dismissed, setDismissed] = useState(false)
  const menu = disabled || dismissed ? null : documentSlash(value, cursor)
  const active = menu ? Math.min(selected, menu.items.length - 1) : 0
  useLayoutEffect(() => {
    if (restore.current !== null && input.current) {
      input.current.focus()
      input.current.setSelectionRange(restore.current, restore.current)
      restore.current = null
    }
  }, [value])
  function insert(block: Block) {
    if (!menu || disabled) return
    const nextCursor = menu.start + block.markdown.length
    restore.current = nextCursor
    setCursor(nextCursor)
    setDismissed(true)
    onChange(value.slice(0, menu.start) + block.markdown + value.slice(menu.end))
  }
  return <>
    <textarea ref={input} className="documents-body" aria-label="Document Markdown" rows={15}
      disabled={disabled} placeholder="Write Markdown… Type / on an empty line to insert a block."
      value={value} onChange={(event) => {
        setCursor(event.target.selectionStart === event.target.selectionEnd ? event.target.selectionStart : -1)
        setSelected(0); setDismissed(false); onChange(event.target.value)
      }} onSelect={(event) => {
        const field = event.currentTarget
        setCursor(field.selectionStart === field.selectionEnd ? field.selectionStart : -1)
      }} onKeyDown={(event) => {
        if (!menu || event.nativeEvent.isComposing || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return
        if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); setDismissed(true) }
        else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          event.preventDefault()
          setSelected((active + (event.key === 'ArrowDown' ? 1 : menu.items.length - 1)) % menu.items.length)
        } else if ((event.key === 'Enter' || event.key === 'Tab') && !event.shiftKey) {
          event.preventDefault(); insert(menu.items[active])
        }
      }} />
    {menu && <div className="documents-blocks fade-in" role="group" aria-label="Insert Markdown block">
      <p className="documents-block-help" role="status">{`Insert content only · /${menu.items[active].command} selected · ↑↓ choose · Enter insert · Esc dismiss`}</p>
      {menu.items.map((block, index) => <Button key={block.command} kind="secondary"
        aria-pressed={index === active} onClick={() => insert(block)}>{`/${block.command} · ${block.label}`}</Button>)}
    </div>}
  </>
}
