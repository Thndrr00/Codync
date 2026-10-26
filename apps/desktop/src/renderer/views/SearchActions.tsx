import { useEffect, useRef, useState } from 'react'
import { Icon } from '../components/Icon'
import { Text } from '../components/Text'
import { useTr } from '../lib/use-language'

export interface SearchAction { id: string; title: string; icon: string; run: () => void }

export function SearchActions({ actions }: { actions: SearchAction[] }) {
  const tr = useTr()
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const list = useRef<HTMLDivElement>(null)
  const results = actions.filter((action) => tr(action.title).toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()) || action.title.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
  useEffect(() => { setActive(0) }, [query])
  useEffect(() => { list.current?.children[active]?.scrollIntoView({ block: 'nearest' }) }, [active])
  const choose = (index: number) => { results[index]?.run() }
  return <div onKeyDown={(event) => {
    if (event.nativeEvent.isComposing || !(event.target instanceof HTMLInputElement)) return
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      setActive((index) => results.length ? (index + (event.key === 'ArrowDown' ? 1 : -1) + results.length) % results.length : 0)
    } else if (event.key === 'Enter') { event.preventDefault(); choose(active) }
  }}>
    <div className="palette-field"><Icon name="bolt" size={15} /><input autoFocus aria-label={tr('Search actions')} placeholder={tr('Search actions')} value={query} onChange={(event) => setQuery(event.target.value)} /></div>
    <div className="palette-list" ref={list} role="listbox" aria-label={tr('Actions')}>
      {results.map((action, index) => <button key={action.id} role="option" aria-selected={index === active} className={`palette-row ${index === active ? 'active' : ''}`} onFocus={() => setActive(index)} onMouseMove={() => setActive(index)} onClick={() => choose(index)}><Icon name={action.icon} size={18} /><Text>{tr(action.title)}</Text></button>)}
      {!results.length ? <Text tone="secondary">No matching actions</Text> : null}
    </div>
  </div>
}
