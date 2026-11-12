import { useMemo } from 'react'
import { ModalHeader, Sheet } from '../../components/Overlay'
import { CardForm } from '../../components/Controls'
import { useStore } from '../../store/context'
import { MaintenanceStore } from '../../store/maintenance-store'
import { useModel } from '../../lib/observable'
import { TokenBudgetSection } from '../settings/TokenBudgetSection'
import { Text } from '../../components/Text'

/** The existing budget editor, scoped to the selected computer rather than global local-host diagnostics. */
export function IslandBudget({ open, botId, onClose }: { open: boolean; botId: string; onClose: () => void }) {
  const store = useStore()
  const maintenance = useMemo(() => new MaintenanceStore(() => store.ready()), [store])
  useModel(maintenance)
  return <Sheet open={open} onClose={onClose} width={640} height={480}><ModalHeader title="Daily token limits" onClose={onClose} /><CardForm>
    {maintenance.error ? <Text tone="danger">{maintenance.error}</Text> : null}
    {open ? <TokenBudgetSection store={maintenance} bots={[...store.bots.values()]} online={store.connection.kind === 'online'} initialScope={botId} /> : null}
  </CardForm></Sheet>
}
