import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import { TableKit } from '@tiptap/extension-table'
import Image from '@tiptap/extension-image'
import type { Editor } from '@tiptap/core'
import { documentSlash } from './document-blocks.ts'

// Preserve Markdown images without making untrusted documents fetch remote resources.
const DocumentImage = Image.extend({
  renderHTML({ node }) { return ['span', { class: 'document-image', 'data-image': node.attrs.src }, node.attrs.alt || 'Image'] },
})

export function documentExtensions() {
  return [
    StarterKit.configure({ underline: false, link: { openOnClick: false, autolink: false, linkOnPaste: false } }),
    Markdown, TaskList, TaskItem.configure({ nested: true }), TableKit, DocumentImage,
  ]
}

export function richDocumentSlash(editor: Editor | null) {
  if (!editor || !editor.isEditable) return null
  const { $from, empty } = editor.state.selection
  if (!empty || $from.parent.type.name !== 'paragraph' || $from.parentOffset !== $from.parent.content.size) return null
  const found = documentSlash($from.parent.textContent, $from.parent.textContent.length)
  return found ? { ...found, from: $from.start(), to: $from.pos } : null
}

export function documentLink(value: string): string | null {
  const text = value.trim()
  try {
    const url = new URL(text)
    return ['https:', 'http:', 'mailto:'].includes(url.protocol) ? text : null
  } catch { return null }
}
