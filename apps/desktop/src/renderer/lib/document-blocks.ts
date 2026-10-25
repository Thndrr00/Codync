/** Editor snippets insert Markdown only; they never execute an action. */
export const DOCUMENT_BLOCKS = [
  { command: 'doc', label: 'Document section', markdown: '## Document\n\n' },
  { command: 'routine', label: 'Routine outline', markdown: '## Routine\n\n- Trigger: \n- Steps: \n' },
  { command: 'skill', label: 'Skill outline', markdown: '## Skill\n\n### Purpose\n\n### Instructions\n\n' },
  { command: 'task', label: 'Task checkbox', markdown: '- [ ] Task\n' },
  { command: 'heading', label: 'Heading', markdown: '## ' },
  { command: 'list', label: 'Bullet list', markdown: '- ' },
  { command: 'code', label: 'Code block', markdown: '```\n\n```\n' },
  { command: 'quote', label: 'Quote', markdown: '> ' },
] as const

export function documentSlash(text: string, cursor: number) {
  if (!Number.isInteger(cursor) || cursor < 0 || cursor > text.length) return null
  if (cursor < text.length && text[cursor] !== '\n' && text[cursor] !== '\r') return null
  const start = text.lastIndexOf('\n', cursor - 1) + 1
  const query = text.slice(start, cursor)
  if (!/^\/[a-z]*$/.test(query)) return null
  let fence: { marker: string; size: number } | null = null
  for (const line of text.slice(0, start).split('\n')) {
    const match = /^ {0,3}(`{3,}|~{3,})(.*)/.exec(line.replace(/\r$/, ''))
    if (!match) continue
    if (!fence) fence = { marker: match[1][0], size: match[1].length }
    else if (match[1][0] === fence.marker && match[1].length >= fence.size && !match[2].trim()) fence = null
  }
  if (fence) return null
  const items = DOCUMENT_BLOCKS.filter((item) => item.command.startsWith(query.slice(1)))
  return items.length ? { start, end: cursor, items } : null
}
