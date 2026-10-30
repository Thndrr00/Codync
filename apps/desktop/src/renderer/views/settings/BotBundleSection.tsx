import { useState } from 'react'
import type { Bot } from '@shared/models'
import { Button, CardSection, ChoicePicker, ValueRow } from '../../components/Controls'
import { Text } from '../../components/Text'
import type { MaintenanceStore } from '../../store/maintenance-store'
import { Field } from '../marketplace/pieces'

/** A bot as a portable text file: copy one to share or back up, or paste one to make its bot here. */
export function BotBundleSection({ store, bots, online }: { store: MaintenanceStore; bots: Bot[]; online: boolean }) {
  const [botId, setBotId] = useState(bots[0]?.id ?? '')
  const [bundle, setBundle] = useState('')
  const [folder, setFolder] = useState('')
  const selected = bots.some((b) => b.id === botId) ? botId : (bots[0]?.id ?? '')
  const copy = async () => {
    const text = await store.exportBot(selected)
    if (text) await navigator.clipboard.writeText(text)
  }
  return (
    <CardSection title="Share a bot"
      footer="A bundle holds the bot's name, instructions, agent, model and fallbacks. It never holds folders, accounts, connectors, history or keys.">
      <ValueRow label="Bot">
        <ChoicePicker selection={selected} options={bots.map((b) => ({ id: b.id, label: b.name }))} onChange={setBotId} />
      </ValueRow>
      <Button kind="secondary" disabled={!online || store.busy || !selected} onClick={() => void copy()}>Copy bundle</Button>
      <Field label="Bundle" value={bundle} onChange={setBundle} lines={[3, 8]} mono placeholder="Paste a bundle here" />
      <Field label="Folder for the imported bots" value={folder} onChange={setFolder} placeholder="Folder path" />
      <Button disabled={!online || store.busy || !bundle.trim() || !folder.trim()} onClick={() => void store.importBots(bundle, folder.trim())}>Import bots</Button>
      {!bots.length ? <Text tone="secondary">No bots on this computer yet.</Text> : null}
    </CardSection>
  )
}
