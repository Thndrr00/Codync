import { Sheet, ModalHeader } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import './shortcuts.css'

const isMac = window.codync.platform === 'darwin'
const key = (value: string) => (isMac ? value.replace('Ctrl', '⌘').replace('Shift', '⇧').replace('Alt', '⌥') : value)

const groups: { title: string; items: [string, string][] }[] = [
  {
    title: 'Everywhere',
    items: [
      [key('Ctrl+K'), 'Search bots, messages and documents'],
      [key('Ctrl+N'), 'New bot'],
      [key('Ctrl+Shift+N'), 'New group'],
      [key('Ctrl+,'), 'Settings'],
      [key('Ctrl+Shift+Space'), 'Bring the chat back, or hide it'],
      ['?', 'This list'],
      ['Esc', 'Close what is open'],
    ],
  },
  {
    title: 'Writing',
    items: [
      [key('Ctrl+Enter'), 'Send without waiting for the button'],
      [key('Ctrl+Z'), 'Undo in the message box'],
      [key('Ctrl+Shift+V'), 'Paste as plain text'],
    ],
  },
  {
    title: 'Conversations',
    items: [
      [key('Ctrl+1…9'), 'Open the nth search result'],
      ['↑ / ↓', 'Move through results'],
      [key('Ctrl+Shift+O'), 'Mark the chat read'],
    ],
  },
  {
    title: 'Canvas and documents',
    items: [
      [key('Ctrl+Scroll'), 'Zoom the canvas'],
      ['Double-click', 'Add a node'],
      [key('Ctrl+S'), 'Save the document or routine'],
    ],
  },
]

/** A single sheet with what the keyboard can do, opened with `?`. */
export function ShortcutsSheet({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Sheet open={open} onClose={onClose} width={560} maxHeight="80vh" dismissOnDim>
      <VStack gap={14}>
        <ModalHeader icon="keyboard.chevron.compact.down" title="Keyboard shortcuts" onClose={onClose} />
        <div className="shortcuts-scroll">
          {groups.map((group) => (
            <section key={group.title} className="shortcuts-group">
              <Text size="caption" tone="secondary" className="shortcuts-title">{group.title}</Text>
              {group.items.map(([keys, what]) => (
                <HStack key={keys} align="center" gap={10} className="shortcuts-row">
                  <kbd className="shortcuts-key">{keys}</kbd>
                  <span className="shortcuts-what">{what}</span>
                </HStack>
              ))}
            </section>
          ))}
        </div>
      </VStack>
    </Sheet>
  )
}
