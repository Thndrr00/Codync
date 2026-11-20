import { useEffect, useRef, useState } from 'react'
import type { Bot } from '@shared/models'
import { draftOf } from '@shared/models'
import { useTr } from '../lib/use-language'
import type { BotStore } from '../store/bot-store'

/**
 * An inline name input for a roster row. Enter and blur save through the existing `save` API;
 * Escape cancels. The caller decides when to mount it (a double-click, never while filtered or
 * dragging) and unmounts it on save or cancel.
 */
export function RosterNameEditor({ bot, store, onDone }: { bot: Bot; store: BotStore; onDone: () => void }) {
  const tr = useTr()
  const [value, setValue] = useState(bot.name)
  const field = useRef<HTMLInputElement>(null)
  const saving = useRef(false)
  const done = useRef(onDone)
  done.current = onDone

  useEffect(() => {
    field.current?.focus()
    field.current?.select()
  }, [])

  const save = () => {
    if (saving.current) return
    saving.current = true
    const name = value.trim()
    // An unchanged or blank name leaves the bot (and its row) as it was.
    if (!name || name === bot.name) return done.current()
    void store.save({ ...draftOf(bot), name })
      .catch((error: unknown) => store.setError(error instanceof Error ? error.message : String(error)))
      .finally(done.current)
  }

  return (
    <input
      ref={field}
      className="roster-name-editor"
      value={value}
      aria-label={tr('Bot name')}
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
        if (event.key === 'Enter') {
          event.preventDefault()
          save()
        } else if (event.key === 'Escape') {
          event.preventDefault()
          done.current()
        }
      }}
    />
  )
}
