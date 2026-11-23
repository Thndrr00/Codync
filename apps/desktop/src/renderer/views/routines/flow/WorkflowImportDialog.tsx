import { useState } from 'react'
import { Button, ChoicePicker } from '../../../components/Controls'
import { ModalHeader } from '../../../components/Overlay'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'
import type { WorkflowImportBody, WorkflowImportInfo } from '../../../store/routine-flow-store'

interface Candidate {
  id: string
  name: string
}

/** Same-named local agents and channels are offered first; an empty choice means "Choose later". */
const match = (names: readonly string[], candidates: readonly Candidate[]): Record<string, string> =>
  Object.fromEntries(names.map((name) => {
    const found = candidates.find((candidate) => candidate.name.toLocaleLowerCase() === name.toLocaleLowerCase())
    return [name, found?.id ?? '']
  }))

/**
 * Import asks which local agent or channel each name in the file means. Connections and media
 * accounts are not in the file, so they are listed under "Connect again"; the host adds the copy
 * paused. Exported workflows carry steps, notes and files, never secrets.
 */
export function WorkflowImportDialog({ info, content, candidates, channels = [], suggestedName, onClose, onImport }: {
  info: WorkflowImportInfo
  content: string
  candidates: readonly Candidate[]
  channels?: readonly Candidate[]
  /** A duplicate arrives pre-named and with the same names already matched. */
  suggestedName?: string
  onClose: () => void
  onImport: (body: WorkflowImportBody) => void
}) {
  const tr = useTr()
  const [name, setName] = useState(suggestedName ?? info.name)
  const [agents, setAgents] = useState(() => match(info.agents, candidates))
  const [chosenChannels, setChosenChannels] = useState(() => match(info.channels, channels))
  const options = (list: readonly Candidate[]) => [
    { id: '', label: 'Choose later' },
    ...list.map((candidate) => ({ id: candidate.id, label: candidate.name })),
  ]
  return (
    <VStack gap={14} className="routine-flow-sheet flow-import-sheet">
      <ModalHeader icon="doc.badge.plus" title={tr('Import {name}').replace('{name}', info.name)} onClose={onClose} />
      <div className="flow-import-scroll">
        <VStack gap={12}>
          <Text size="callout" tone="secondary">{tr('The copy starts paused.')}</Text>
          <label className="flow-import-name">
            <Text size="caption" tone="secondary">{tr('Workflow name')}</Text>
            <input className="routine-input" value={name} maxLength={120} onChange={(event) => setName(event.target.value)} />
          </label>
          {info.agents.length > 0 ? (
            <VStack gap={6}>
              <Text size="caption" tone="secondary" weight="semibold">{tr('Agents')}</Text>
              {info.agents.map((agent) => (
                <HStack key={agent} gap={8} align="center">
                  <Text size="callout" grow truncate title={agent}>{agent}</Text>
                  <ChoicePicker
                    selection={agents[agent] ?? ''}
                    options={options(candidates)}
                    onChange={(id) => setAgents({ ...agents, [agent]: id })}
                  />
                </HStack>
              ))}
            </VStack>
          ) : null}
          {info.channels.length > 0 ? (
            <VStack gap={6}>
              <Text size="caption" tone="secondary" weight="semibold">{tr('Channels')}</Text>
              {info.channels.map((channel) => (
                <HStack key={channel} gap={8} align="center">
                  <Text size="callout" grow truncate title={channel}>{channel}</Text>
                  <ChoicePicker
                    selection={chosenChannels[channel] ?? ''}
                    options={options(channels)}
                    onChange={(id) => setChosenChannels({ ...chosenChannels, [channel]: id })}
                  />
                </HStack>
              ))}
            </VStack>
          ) : null}
          {info.links.length > 0 ? (
            <VStack gap={4}>
              <Text size="caption" tone="secondary" weight="semibold">{tr('Connect again')}</Text>
              {info.links.map((link, index) => (
                <Text key={`${link.step}-${index}`} size="callout" tone="secondary">
                  {tr('{step}: {app}').replace('{step}', link.step).replace('{app}', link.app)}
                </Text>
              ))}
            </VStack>
          ) : null}
          {info.assets > 0 ? (
            <Text size="callout" tone="secondary">{tr('Files to upload: {count}').replace('{count}', String(info.assets))}</Text>
          ) : null}
        </VStack>
      </div>
      <HStack gap={8}>
        <Spacer />
        <Button kind="secondary" onClick={onClose}>{tr('Cancel')}</Button>
        <Button
          disabled={!name.trim()}
          onClick={() => onImport({ name: name.trim(), content, agents, channels: chosenChannels })}
        >
          {tr('Import')}
        </Button>
      </HStack>
    </VStack>
  )
}
