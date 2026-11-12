import { useMemo } from 'react'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import './a2ui.css'

/**
 * Declarative agent UI: a fenced ```a2ui block whose body is JSON. The agent describes what it
 * wants (text, facts, a table, bars) and Codync draws it with its own components. The schema is a
 * closed list: nothing here executes code or renders HTML from the model.
 */
interface A2uiBlockText { type: 'text'; text: string }
interface A2uiFact { label: string; value: string }
interface A2uiFacts { type: 'facts'; items: A2uiFact[] }
interface A2uiTable { type: 'table'; columns: string[]; rows: string[][] }
interface A2uiBars { type: 'bars'; items: { label: string; value: number }[] }
interface A2uiNote { type: 'note'; text: string; tone?: 'info' | 'attention' | 'danger' }
type Block = A2uiBlockText | A2uiFacts | A2uiTable | A2uiBars | A2uiNote

interface Document {
  title?: string
  blocks?: Block[]
}

const limited = (text: unknown, max = 2_000): string => (typeof text === 'string' ? text.slice(0, max) : '')

function parse(text: string): Document | null {
  try {
    const value = JSON.parse(text) as Document
    if (!value || typeof value !== 'object' || !Array.isArray(value.blocks)) return null
    return value
  } catch {
    return null
  }
}

export function A2uiBlock({ text }: { text: string }) {
  const doc = useMemo(() => parse(text), [text])
  if (!doc) {
    return (
      <div className="a2ui a2ui-invalid">
        <Icon name="exclamationmark.triangle" size={13} />
        <Text size="caption" tone="danger">This a2ui block is not valid JSON.</Text>
      </div>
    )
  }
  const blocks = (doc.blocks ?? []).slice(0, 24)
  return (
    <section className="a2ui" aria-label={doc.title ? limited(doc.title, 120) : 'Agent interface'}>
      {doc.title ? (
        <header className="a2ui-title">
          <Text size="body" weight="semibold">{limited(doc.title, 120)}</Text>
        </header>
      ) : null}
      <div className="a2ui-body">
        {blocks.map((block, index) => {
          const key = `${block.type}-${index}`
          switch (block.type) {
            case 'text':
              return <p key={key} className="a2ui-text">{limited(block.text, 4_000)}</p>
            case 'note':
              return (
                <div key={key} className="a2ui-note" data-tone={block.tone ?? 'info'}>
                  <Icon name={block.tone === 'danger' ? 'exclamationmark.circle' : 'exclamationmark.circle'} size={13} />
                  <span>{limited(block.text, 1_500)}</span>
                </div>
              )
            case 'facts':
              return (
                <dl key={key} className="a2ui-facts">
                  {(block.items ?? []).slice(0, 12).map((item, i) => (
                    <div key={`${key}-${i}`} className="a2ui-fact">
                      <dt>{limited(item.label, 80)}</dt>
                      <dd>{limited(item.value, 400)}</dd>
                    </div>
                  ))}
                </dl>
              )
            case 'table': {
              const columns = (block.columns ?? []).slice(0, 8).map((c) => limited(c, 80))
              if (columns.length === 0) return null
              return (
                <div key={key} className="a2ui-table-scroll">
                  <table className="a2ui-table">
                    <thead>
                      <tr>{columns.map((column, i) => <th key={i}>{column}</th>)}</tr>
                    </thead>
                    <tbody>
                      {(block.rows ?? []).slice(0, 50).map((row, r) => (
                        <tr key={r}>
                          {columns.map((_, c) => <td key={c}>{limited(row?.[c], 400)}</td>)}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )
            }
            case 'bars': {
              const items = (block.items ?? []).slice(0, 12)
              const max = Math.max(1, ...items.map((item) => (Number.isFinite(item.value) ? item.value : 0)))
              return (
                <VStack key={key} gap={6} className="a2ui-bars">
                  {items.map((item, i) => (
                    <div key={i} className="a2ui-bar">
                      <Text size="caption" tone="secondary" className="a2ui-bar-label">{limited(item.label, 80)}</Text>
                      <div className="a2ui-bar-track">
                        <div className="a2ui-bar-fill" style={{ width: `${Math.min(100, Math.max(0, (item.value / max) * 100))}%` }} />
                      </div>
                      <Text size="caption" tabular>{Number.isFinite(item.value) ? item.value : 0}</Text>
                    </div>
                  ))}
                </VStack>
              )
            }
            default:
              return null
          }
        })}
      </div>
      <footer className="a2ui-footer">
        <HStack gap={5} align="center">
          <Icon name="square.grid.2x2" size={11} />
          <Text size="caption" tone="tertiary">Agent interface</Text>
        </HStack>
      </footer>
    </section>
  )
}
