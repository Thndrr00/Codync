import type { IslandThreadDestination } from '../../store/island-thread-navigation'
import { Sheet } from '../../components/Overlay'
import { SetupTerminalView } from '../marketplace/SetupTerminal'

/** Attaches to the retained setup session inside the selected computer's existing store context. */
export function IslandTerminal({ terminal, onClose }: { terminal: Extract<IslandThreadDestination, { kind: 'terminal' }> | null; onClose: () => void }) {
  return <Sheet open={terminal !== null} onClose={onClose} width={720} height={500}>{terminal ? <SetupTerminalView backend={terminal.backend} step={terminal.step} existingTermId={terminal.id} back={onClose} /> : null}</Sheet>
}
