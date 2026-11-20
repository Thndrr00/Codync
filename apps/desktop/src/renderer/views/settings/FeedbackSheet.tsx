import { useState } from 'react'
import { Button, ChoicePicker } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { liveClient } from './parts'

type FeedbackKind = 'bug' | 'idea'

/**
 * Writes a bug or an idea here and sends it straight to the host, which posts it as a public
 * GitHub issue. The dialog is the approval — there is no second card and no system dialog.
 */
export function SendFeedbackButton() {
  const tr = useTr()
  const store = useStore()
  const [open, setOpen] = useState(false)
  const [kind, setKind] = useState<FeedbackKind>('bug')
  const [title, setTitle] = useState('')
  const [text, setText] = useState('')
  const [details, setDetails] = useState('')
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null)

  const send = () => {
    const client = liveClient(store)
    if (!client) {
      setResult({ ok: false, text: tr('Not connected to {name}.').replaceAll('{name}', () => store.hostName) })
      return
    }
    setBusy(true)
    setResult(null)
    void client.call<{ url: string }>('sendFeedback', { kind, title, text, details })
      .then((answer) => {
        setResult({ ok: true, text: tr('Your report is public at {url}').replaceAll('{url}', () => answer.url ?? '') })
        setTitle('')
        setText('')
        setDetails('')
      })
      .catch((error: unknown) => setResult({ ok: false, text: error instanceof Error ? error.message : String(error) }))
      .finally(() => setBusy(false))
  }

  return (
    <>
      <VStack gap={6}>
        <Button kind="secondary" onClick={() => setOpen(true)}>{tr('Send feedback')}</Button>
        {result ? <Text size="footnote" tone={result.ok ? 'secondary' : 'attention'}>{result.text}</Text> : null}
      </VStack>
      <Dialog
        open={open}
        title={tr('Send feedback')}
        message={tr('Bug reports and ideas become public GitHub issues. Write them in English.')}
        actions={[{ title: busy ? tr('Sending…') : tr('Send'), disabled: busy || !title.trim() || !text.trim(), action: send }]}
        onClose={() => setOpen(false)}
      >
        <VStack gap={8}>
          <HStack as="span" gap={10}>
            <Text size="caption" tone="secondary">{tr('Type')}</Text>
            <ChoicePicker
              selection={kind}
              options={[{ id: 'bug', label: tr('Bug') }, { id: 'idea', label: tr('Idea') }]}
              onChange={setKind}
            />
          </HStack>
          <input
            className="dialog-field"
            data-autofocus
            aria-label={tr('Title')}
            placeholder={tr('Title')}
            value={title}
            spellCheck={false}
            onChange={(e) => setTitle(e.target.value)}
          />
          <textarea
            className="dialog-field dialog-textarea"
            aria-label={tr('Report')}
            placeholder={tr('What went wrong, or what you want')}
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
          <textarea
            className="dialog-field dialog-textarea"
            aria-label={tr('Details')}
            placeholder={tr('Steps to reproduce, or the problem it solves (optional)')}
            value={details}
            onChange={(e) => setDetails(e.target.value)}
          />
        </VStack>
      </Dialog>
    </>
  )
}
