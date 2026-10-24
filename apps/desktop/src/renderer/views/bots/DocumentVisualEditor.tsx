import { useEffect, useMemo, useRef, useState } from 'react'
import { EditorContent, useEditor } from '@tiptap/react'
import { Button } from '../../components/Controls'
import { documentExtensions, documentLink, richDocumentSlash } from '../../lib/document-editor'
import type { DOCUMENT_BLOCKS } from '../../lib/document-blocks'

type Block = typeof DOCUMENT_BLOCKS[number]
const EDITOR_PROPS = { attributes: { role: 'textbox', 'aria-label': 'Visual document editor', 'aria-multiline': 'true', class: 'documents-rich' } }

/** Markdown remains the stored representation; only user document edits serialize it. */
export default function DocumentVisualEditor({ value, disabled, onChange }: {
  value: string; disabled: boolean; onChange: (value: string) => void
}) {
  const latest = useRef({ onChange })
  latest.current = { onChange }
  const last = useRef(value)
  const [, render] = useState(0)
  const [selected, setSelected] = useState(0)
  const [dismissed, setDismissed] = useState(false)
  const [link, setLink] = useState<string | null>(null)
  const [linkError, setLinkError] = useState(false)
  const extensions = useMemo(documentExtensions, [])
  const editor = useEditor({
    extensions, content: value, contentType: 'markdown', editable: !disabled,
    immediatelyRender: false,
    editorProps: EDITOR_PROPS,
    onUpdate({ editor }) {
      const markdown = editor.getMarkdown()
      last.current = markdown
      latest.current.onChange(markdown)
      setDismissed(false); setSelected(0)
    },
  })
  useEffect(() => {
    if (!editor) return
    const changed = () => render((n) => n + 1)
    editor.on('transaction', changed)
    return () => { editor.off('transaction', changed) }
  }, [editor])
  useEffect(() => { editor?.setEditable(!disabled, false) }, [editor, disabled])
  useEffect(() => {
    if (editor && value !== last.current) {
      editor.commands.setContent(value, { contentType: 'markdown', emitUpdate: false })
      last.current = value
    }
  }, [editor, value])
  const menu = dismissed ? null : richDocumentSlash(editor)
  const active = menu ? Math.min(selected, menu.items.length - 1) : 0
  function insert(block: Block) {
    if (!editor || !menu || disabled) return
    editor.chain().focus().deleteRange({ from: menu.from, to: menu.to }).insertContent(block.markdown, { contentType: 'markdown' }).run()
    setDismissed(true)
  }
  function applyLink() {
    if (!editor || link === null) return
    const href = documentLink(link)
    if (!href) { setLinkError(true); return }
    const selection = editor.state.selection
    if (selection.empty && !editor.isActive('link')) editor.chain().focus().insertContent({ type: 'text', text: href, marks: [{ type: 'link', attrs: { href } }] }).run()
    else editor.chain().focus().extendMarkRange('link').setLink({ href }).run()
    setLink(null); setLinkError(false)
  }
  if (!editor) return <p role="status">Opening visual editor…</p>
  return <div className="documents-visual" onKeyDownCapture={(event) => {
    if (!editor.view.dom.contains(event.target as Node)) return
    if (!menu || event.nativeEvent.isComposing || event.shiftKey || event.ctrlKey || event.metaKey || event.altKey) return
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); setDismissed(true) }
    else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault(); event.stopPropagation()
      setSelected((active + (event.key === 'ArrowDown' ? 1 : menu.items.length - 1)) % menu.items.length)
    } else if (event.key === 'Enter' || event.key === 'Tab') { event.preventDefault(); event.stopPropagation(); insert(menu.items[active]) }
  }}>
    <div className="documents-blocks" role="group" aria-label="Document formatting">
      <Button kind="secondary" disabled={disabled} aria-pressed={editor.isActive('bold')} onClick={() => editor.chain().focus().toggleBold().run()}>Bold</Button>
      <Button kind="secondary" disabled={disabled} aria-pressed={editor.isActive('italic')} onClick={() => editor.chain().focus().toggleItalic().run()}>Italic</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}>Heading</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => editor.chain().focus().toggleBulletList().run()}>List</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => editor.chain().focus().toggleOrderedList().run()}>Numbered list</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => editor.chain().focus().toggleTaskList().run()}>Task</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => editor.chain().focus().toggleBlockquote().run()}>Quote</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => editor.chain().focus().toggleCodeBlock().run()}>Code</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => { setLink(editor.getAttributes('link').href ?? ''); setLinkError(false) }}>Link</Button>
      <Button kind="secondary" disabled={disabled || !editor.can().undo()} onClick={() => editor.chain().focus().undo().run()}>Undo</Button>
      <Button kind="secondary" disabled={disabled || !editor.can().redo()} onClick={() => editor.chain().focus().redo().run()}>Redo</Button>
    </div>
    {link !== null && <div className="documents-blocks fade-in">
      <input className="documents-title" aria-label="Link URL" placeholder="https://example.com" value={link} onChange={(event) => setLink(event.target.value)} disabled={disabled} />
      {linkError && <p role="alert">Enter an HTTP, HTTPS or mailto link.</p>}
      <Button disabled={disabled} onClick={applyLink}>Apply link</Button>
      <Button kind="secondary" disabled={disabled} onClick={() => { editor.chain().focus().unsetLink().run(); setLink(null) }}>Remove link</Button>
      <Button kind="secondary" onClick={() => setLink(null)}>Cancel</Button>
    </div>}
    <EditorContent editor={editor} />
    {menu && <div className="documents-blocks fade-in" role="group" aria-label="Insert document block">
      <p className="documents-block-help" role="status">{`/${menu.items[active].command} selected · ↑↓ choose · Enter insert · Esc dismiss`}</p>
      {menu.items.map((block) => <Button key={block.command} kind="secondary" onClick={() => insert(block)}>{`/${block.command} · ${block.label}`}</Button>)}
    </div>}
    <p className="documents-block-help">Type / in an empty paragraph to insert a block. Content is saved as Markdown.</p>
  </div>
}
