import { useState } from 'react'
import { Button, CardForm, CardSection, ChoiceList, IconButton } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import type { Brain } from '../../../shared/models'
import { useTr } from '../../lib/use-language'
import type { MemoryStore } from '../../store/memory-store'
import { FormSheet, Field } from '../marketplace/pieces'
import './memory.css'

/** The brains a bot can work in: the Fleet one (the memory all bots used to share) and the projects, to pick one, make one, rename it, delete it, or move a brain to another computer. */
export function MemoryBrainSheet({ memory, open, onClose }: { memory: MemoryStore; open: boolean; onClose: () => void }) {
  const tr = useTr()
  const [name, setName] = useState('')
  const [renaming, setRenaming] = useState<Brain | null>(null)
  const [pasted, setPasted] = useState('')
  const [notice, setNotice] = useState<string | null>(null)
  const reset = () => { setRenaming(null); setName(''); setPasted(''); setNotice(null) }
  const brains = memory.brains ?? []
  const current = memory.view?.brain ?? null

  const save = async () => {
    const trimmed = name.trim()
    if (!trimmed) return
    await memory.saveBrain(trimmed, renaming?.id)
    setName('')
    setRenaming(null)
  }

  return <FormSheet open={open} onClose={() => { reset(); onClose() }} height={620}>
    <ModalHeader title="Brain" />
    <CardForm>
      {memory.error ? <CardSection><p role="alert" className="memory-error">{memory.error}</p></CardSection> : null}
      <CardSection footer="A brain is memory bots share: a project's, or the Fleet one, which every bot that uses the shared memory reads.">
        <ChoiceList selection={current && !current.shared ? current.id : 'fleet'} options={brains.map((brain) => ({
          id: brain.id,
          label: brain.shared ? tr('Fleet (every bot)') : brain.name,
          detail: brain.shared
            ? tr('What your bots share. Joining keeps the memory you already share.')
            : tr('{n} bots').replace('{n}', String(brain.members?.length ?? 0)),
        }))} onChange={(id) => {
          const brain = brains.find((b) => b.id === id)
          if (brain) void memory.joinBrain(brain)
        }} />
        {current ? <Button kind="secondary" onClick={() => void memory.leaveBrain()}>
          {tr('Leave this brain')}
        </Button> : null}
      </CardSection>
      <CardSection title={renaming ? tr('Rename {name}').replace('{name}', renaming.name) : tr('New brain')}
        accessory={renaming ? <IconButton title="Cancel rename" icon="xmark" onClick={() => setRenaming(null)} /> : undefined}>
        <Field value={name} onChange={setName} placeholder="Project name" label={tr('Brain name')} />
        <Button onClick={() => void save()} disabled={!name.trim()}>{renaming ? tr('Rename') : tr('Create')}</Button>
      </CardSection>
      {brains.filter((brain) => !brain.shared).map((brain) => <CardSection key={brain.id} title={brain.name}>
        <HStack gap={2}>
          <IconButton title={tr('Rename {name}').replace('{name}', brain.name)} icon="pencil" onClick={() => { setRenaming(brain); setName(brain.name) }} />
          <IconButton title={tr('Export {name}').replace('{name}', brain.name)} icon="arrow.down.to.line" onClick={() => void exportBrain(memory, brain, tr, setNotice)} />
          <IconButton title={tr('Delete {name}').replace('{name}', brain.name)} icon="trash" onClick={() => void memory.deleteBrain(brain)} />
          <Text size="caption" tone="secondary">{tr('Deleting a brain keeps its bots and their own memory; what the brain held goes.')}</Text>
        </HStack>
      </CardSection>)}
      <CardSection title={tr('Import a brain')} footer={notice ?? undefined}>
        <Field value={pasted} onChange={setPasted} placeholder="Paste a brain file" lines={[3, 6]} mono />
        <Button disabled={!pasted.trim()} onClick={() => void importBrain(memory, pasted, tr, setNotice, setPasted)}>
          {tr('Import')}
        </Button>
      </CardSection>
      <VStack gap={0}>
        <Text size="caption" tone="tertiary"><Icon name="brain" size={10} /> {tr('Bots in a brain read its memory, and no other project\'s.')}</Text>
      </VStack>
    </CardForm>
  </FormSheet>
}

/** Saves a brain as a file to give another copy of Codync. */
async function exportBrain(memory: MemoryStore, brain: Brain, tr: (text: string) => string, noticed: (text: string) => void) {
  try {
    const text = await memory.exportBrain(brain)
    const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }))
    const link = document.createElement('a')
    link.href = url
    link.download = `${brain.name}.brain.json`
    link.click()
    URL.revokeObjectURL(url)
    noticed(tr('Saved {name}.').replace('{name}', `${brain.name}.brain.json`))
  } catch (error) {
    noticed(error instanceof Error ? error.message : String(error))
  }
}

/** A brain made from a pasted file; what the checks turned away is told. */
async function importBrain(memory: MemoryStore, pasted: string, tr: (text: string) => string, noticed: (text: string) => void, clear: (text: string) => void) {
  try {
    const skipped = await memory.importBrain(pasted)
    clear('')
    noticed(skipped
      ? tr('Imported, {n} files were turned away.').replace('{n}', String(skipped))
      : tr('Imported.'))
  } catch (error) {
    noticed(error instanceof Error ? error.message : String(error))
  }
}
