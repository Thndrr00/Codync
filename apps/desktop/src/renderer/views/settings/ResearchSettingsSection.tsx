import { useEffect, useMemo, useState } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { ResearchSettingsStore } from '../../store/research-settings-store'
import { ErrorText, Field } from '../marketplace/pieces'

/** Local Windows credentials; other clients show status and use the host's chat tool. */
export function ResearchSettingsSection() {
  const app = useApp()
  const local = app.local
  const model = useModel(useMemo(() => new ResearchSettingsStore(async () => {
    if (!local) throw new Error('Start this computer’s host to configure research.')
    return local.ready()
  }), [local]))
  const [key, setKey] = useState('')
  const [remove, setRemove] = useState<ResearchSettingsStore | null>(null)
  useEffect(() => {
    setKey(''); setRemove(null); model.resume(); void model.load()
    return () => model.dispose()
  }, [model])
  const save = async () => { const value = key; setKey(''); await model.save(value) }
  return <CardSection title="Research with sources" footer="The Parallel key stays in this computer’s credential vault. Research sends your question to Parallel and uses its normal API pricing. Ask a connected bot to research; it will return source links and evidence.">
    <Text>{model.configured === null ? 'Research status has not loaded.' : model.configured ? 'Parallel key is saved.' : 'Add a Parallel key to enable research.'}</Text>
    <Field label="Parallel API key" secret value={key} onChange={setKey} placeholder="Paste a key to add or replace it" />
    <Button disabled={model.busy || !key.trim() || !local} onClick={() => void save()}>{model.busy ? 'Saving…' : 'Save research key'}</Button>
    <Button kind="secondary" disabled={model.busy || model.loading || !local} onClick={() => void model.load()}>Refresh research status</Button>
    {model.configured && <Button kind="secondary" disabled={model.busy} onClick={() => setRemove(model)}>Remove research key</Button>}
    {model.error && <ErrorText text={model.error} />}
    {model.notice && <Text as="div" role="status">{model.notice}</Text>}
    <Dialog open={remove === model} title="Remove research key?" message="New web research requests will require a key. Captured evidence packets remain available." actions={[{ title: 'Remove key', action: () => { const target = remove; setRemove(null); if (target === model) void target.save('') } }]} onClose={() => setRemove(null)} />
  </CardSection>
}
