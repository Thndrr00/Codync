import { useTr } from '../lib/use-language'
import { prefs, usePref } from '../lib/prefs'
import { ModalHeader } from './Overlay'
import { VStack } from './Stack'
import { Text } from './Text'
import { KEYBOARD_SHORTCUTS, shortcutKeys } from './keyboard-shortcuts'
import './keyboard-cheatsheet.css'

/** The `?` sheet: every row comes from KEYBOARD_SHORTCUTS, showing the bindings this device uses. */
export function KeyboardCheatsheet({ onClose }: { onClose?: () => void }) {
  const tr = useTr()
  const [bindings] = usePref(prefs.keyboardBindings)
  const keysFor = (id: string) => shortcutKeys(id, bindings, window.codync.platform) ?? ''
  const groups = [...new Set(KEYBOARD_SHORTCUTS.map((shortcut) => shortcut.group))].map((name) => ({
    name,
    rows: KEYBOARD_SHORTCUTS.filter((shortcut) => shortcut.group === name),
  }))
  return (
    <VStack gap={0} className="keyboard-cheatsheet">
      <ModalHeader icon="info.circle" title={tr('Keyboard shortcuts')} subtitle={tr('The shortcuts available in this window.')} onClose={onClose ?? null} />
      <div className="keyboard-cheatsheet-body">
        {groups.map((group) => (
          <section key={group.name} className="keyboard-cheatsheet-group">
            <Text as="h3" size="caption" weight="semibold" tone="secondary">{tr(group.name)}</Text>
            {group.rows.map((shortcut) => (
              <div key={shortcut.id} className="keyboard-cheatsheet-row">
                <Text size="body" tone="text" grow>{tr(shortcut.label)}</Text>
                <span className="keyboard-cheatsheet-keys" aria-label={keysFor(shortcut.id)}>{keysFor(shortcut.id)}</span>
              </div>
            ))}
          </section>
        ))}
      </div>
    </VStack>
  )
}
