import type { MediaKind, MediaModel } from '@shared/media'
import { DropdownMenu } from '../../../components/Controls'
import { Text } from '../../../components/Text'
import {
  availableMediaModels, unavailableMediaModel, type FlowMediaCatalog,
} from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'

/**
 * The media step's model button. A saved model the host can't make keeps its name and a
 * "Model not available" marker; the menu lists the available models and still shows the saved one,
 * so it can be replaced. The media switch-all change needs no UI beyond this availability flag.
 */
export function MediaModelButton({ answer, kind, modelId, onPick }: {
  answer: FlowMediaCatalog | null
  kind: MediaKind
  modelId: string
  onPick: (model: MediaModel) => void
}) {
  const tr = useTr()
  const available = availableMediaModels(answer, kind)
  const saved = unavailableMediaModel(answer, modelId, kind)
  const name = saved?.name
    ?? available.find((model) => model.id === modelId)?.name
    ?? answer?.models.find((model) => model.id === modelId)?.name
    ?? modelId
  const items = [
    ...available.map((model) => ({ title: model.name, selected: model.id === modelId, action: () => onPick(model) })),
    ...(saved && !available.some((model) => model.id === modelId)
      ? [{ title: `${saved.name} · ${tr('Model not available')}`, selected: true, disabled: true, action: () => {} }]
      : []),
  ]
  return (
    <>
      <DropdownMenu className="flow-menu-button" items={() => items}>
        <Text>{name}</Text>
      </DropdownMenu>
      {saved ? <Text size="caption" tone="attention">{tr('Model not available')}</Text> : null}
    </>
  )
}
