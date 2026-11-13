import { useCallback, useEffect, useState } from 'react'
import type { ComfyTemplate, ComfyTemplateSummary, CuratedTemplate, TemplateInput, TemplatePreflightProblem } from '@shared/tools'
import { Button, CardSection, ChoicePicker, RowLabel, Spinner, Switch } from '../../../components/Controls'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'
import { analytics } from '../../../store/analytics'
import { ErrorText } from '../pieces'
import { ProblemRow, type ToolCall } from './parts'

/** What a template makes, in one word for the badge. */
function outputLabel(kind: ComfyTemplateSummary['output']): string {
  return { image: 'Image', video: 'Video', audio: 'Audio', mesh: '3D' }[kind]
}

/** The form's first values: the manifest's defaults, or an empty field the host fills. */
function initialValues(inputs: TemplateInput[]): Record<string, unknown> {
  const values: Record<string, unknown> = {}
  for (const input of inputs) {
    if (input.default !== undefined && input.default !== null) values[input.key] = input.default
    else if (input.type === 'boolean') values[input.key] = false
    else if (input.type === 'seed') values[input.key] = ''
    else if (input.type === 'number' || input.type === 'integer') values[input.key] = input.min ?? 0
    else values[input.key] = ''
  }
  return values
}

/** Empty optional fields are left out so the host applies the manifest's default (and randomizes a seed). */
function payload(values: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(Object.entries(values).filter(([, value]) => value !== '' && value !== undefined && value !== null))
}

function base64(bytes: Uint8Array): string {
  let binary = ''
  const chunk = 0x8000
  for (let offset = 0; offset < bytes.length; offset += chunk) {
    binary += String.fromCharCode(...Array.from(bytes.subarray(offset, offset + chunk)))
  }
  return btoa(binary)
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function seconds(value: number | null | undefined): string | null {
  if (!value) return null
  return value < 60 ? `${value}s` : `${Math.round(value / 60)} min`
}

function Field({ input, value, picked, busy, onChange, onPick }: {
  input: TemplateInput
  value: unknown
  picked?: string
  busy: boolean
  onChange: (value: unknown) => void
  onPick: () => void
}) {
  const tr = useTr()
  const text = typeof value === 'string' || typeof value === 'number' ? String(value) : ''
  return (
    <VStack gap={6}>
      <RowLabel label={input.label} detail={input.help} />
      {input.type === 'select' ? (
        <ChoicePicker
          selection={text}
          options={(input.options ?? []).map((option) => ({ id: option, label: option }))}
          onChange={onChange}
        />
      ) : input.type === 'boolean' ? (
        <Switch on={value === true} onChange={onChange} />
      ) : input.type === 'image' ? (
        <HStack gap={8}>
          <Button kind="secondary" disabled={busy} onClick={onPick}>{tr(picked ? 'Choose another picture' : 'Choose a picture')}</Button>
          {picked ? <Text size="caption" tone="secondary">{picked}</Text> : text ? <Text size="caption" tone="secondary">{text}</Text> : null}
        </HStack>
      ) : input.type === 'seed' ? (
        <HStack gap={8}>
          <input
            className="field-box comfy-number"
            type="number"
            value={text}
            placeholder={tr('Random')}
            onChange={(event) => onChange(event.target.value)}
          />
          <Button kind="secondary" onClick={() => onChange('')}>{tr('Random')}</Button>
        </HStack>
      ) : input.type === 'number' || input.type === 'integer' ? (
        <input
          className="field-box comfy-number"
          type="number"
          value={text}
          min={input.min ?? undefined}
          max={input.max ?? undefined}
          step={input.step ?? (input.type === 'integer' ? 1 : 'any')}
          onChange={(event) => onChange(event.target.value)}
        />
      ) : input.key === 'prompt' || input.key === 'negative' ? (
        <textarea className="field-box comfy-text" rows={3} value={text} onChange={(event) => onChange(event.target.value)} />
      ) : (
        <input className="field-box" value={text} onChange={(event) => onChange(event.target.value)} />
      )}
    </VStack>
  )
}

/** The gallery and the form a template's manifest generates: pick, fill, run. */
export function ComfyTemplates({ call, onQueued }: { call: ToolCall; onQueued: () => void }) {
  const tr = useTr()
  const [templates, setTemplates] = useState<ComfyTemplateSummary[] | null>(null)
  const [curated, setCurated] = useState<CuratedTemplate[]>([])
  const [selected, setSelected] = useState<ComfyTemplate | null>(null)
  const [values, setValues] = useState<Record<string, unknown>>({})
  const [picked, setPicked] = useState<Record<string, string>>({})
  const [error, setError] = useState<string | null>(null)
  const [note, setNote] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(() => {
    void call<{ templates: ComfyTemplateSummary[]; curated?: CuratedTemplate[] }>('comfyTemplates')
      .then((reply) => {
        setTemplates(reply.templates)
        setCurated(reply.curated ?? [])
      })
      .catch((e: unknown) => setError(message(e)))
  }, [call])

  useEffect(load, [load])

  const open = async (id: string) => {
    setBusy(true)
    setError(null)
    try {
      const template = await call<ComfyTemplate>('comfyTemplate', { id })
      setSelected(template)
      setValues(initialValues(template.inputs))
      setPicked({})
    } catch (e) {
      setError(message(e))
    } finally {
      setBusy(false)
    }
  }

  const choose = async (input: TemplateInput) => {
    setError(null)
    try {
      const files = await window.codync.app.pickFiles()
      const file = files.find((candidate) => /\.(png|jpe?g|webp|bmp|gif|wav|mp3|flac|ogg|mp4|webm|mov)$/i.test(candidate.name))
      if (!file) return
      setBusy(true)
      const stored = await call<{ name: string }>('comfyUploadImage', { name: file.name, data: base64(file.data) }, 120_000)
      setValues((current) => ({ ...current, [input.key]: stored.name }))
      setPicked((current) => ({ ...current, [input.key]: file.name }))
    } catch (e) {
      setError(message(e))
    } finally {
      setBusy(false)
    }
  }

  const act = async (kind: string, problem: TemplatePreflightProblem) => {
    setError(null)
    try {
      if (kind === 'download' && problem.action.folder && problem.action.file) {
        await call('comfyModelDownload', { folder: problem.action.folder, file: problem.action.file })
        setNote(tr('The download started: watch it in Models.'))
      } else if (kind === 'start') {
        await call('toolStart')
      } else if (kind === 'openUrl' && typeof problem.action.url === 'string') {
        window.codync.app.openExternal(problem.action.url)
      } else if (kind === 'installNode') {
        await window.codync.tools.open('comfy')
      }
    } catch (e) {
      setError(message(e))
    }
  }

  const run = async () => {
    if (!selected) return
    setBusy(true)
    setError(null)
    try {
      const reply = await call<{ ok?: boolean; problems?: TemplatePreflightProblem[] }>('comfyRunTemplate', {
        id: selected.id,
        values: payload(values),
      })
      if (reply.ok === false) {
        analytics.track({ event: 'comfy_template_run', outcome: 'refused', output: selected.output.type })
        setError(reply.problems?.[0]?.message ?? tr('The job was not queued.'))
        return
      }
      analytics.track({ event: 'comfy_template_run', outcome: 'queued', output: selected.output.type })
      onQueued()
    } catch (e) {
      analytics.track({ event: 'comfy_template_run', outcome: 'failed', output: selected.output.type })
      setError(message(e))
    } finally {
      setBusy(false)
    }
  }

  if (!selected) {
    return (
      <VStack gap={12}>
        {templates === null ? (
          <HStack gap={8}><Spinner size={18} /><Text tone="secondary">{tr('Reading the templates…')}</Text></HStack>
        ) : templates.length === 0 ? (
          <Text size="caption" tone="secondary">{tr('No templates are installed yet. Restart the host or add one under the templates folder.')}</Text>
        ) : (
          <div className="comfy-gallery">
            {templates.map((template) => (
              <button key={template.id} className="comfy-template" onClick={() => void open(template.id)}>
                <HStack gap={8}>
                  <Text weight="medium" tone="text" grow>{template.name}</Text>
                  <span className="comfy-badge">{tr(outputLabel(template.output))}</span>
                </HStack>
                <Text size="caption" tone="secondary">{template.description}</Text>
                <HStack gap={10} wrap>
                  {seconds(template.estimateSeconds) ? <Text size="caption" tone="secondary">{tr('About {time}').replace('{time}', seconds(template.estimateSeconds) ?? '')}</Text> : null}
                  {template.vramGb ? <Text size="caption" tone="secondary">{tr('{gb} GB of graphics memory').replace('{gb}', String(template.vramGb))}</Text> : null}
                  {template.models > 0 ? <Text size="caption" tone="secondary">{tr('{count} models').replace('{count}', String(template.models))}</Text> : null}
                </HStack>
              </button>
            ))}
          </div>
        )}
        {curated.length > 0 ? (
          <VStack gap={8} className="comfy-challenge">
            <Text size="caption" tone="secondary">
              {tr('Open Source Higgsfield: each entry is its author\'s repository. Import its graph, nodes and models from there; it is not installed here.')}
            </Text>
            {curated.filter((entry) => !entry.imported).map((entry) => (
              <button key={entry.id} className="comfy-template curated" onClick={() => window.codync.app.openExternal(entry.repo)}>
                <HStack gap={8}>
                  <Text weight="medium" tone="text" grow>{entry.name}</Text>
                  <span className="comfy-badge">{entry.license}</span>
                </HStack>
                <Text size="caption" tone="secondary">{entry.summary}</Text>
                <HStack gap={10} wrap>
                  <Text size="caption" tone="tertiary">{entry.author}</Text>
                  <Text size="caption" tone="tertiary">{entry.output.join(' · ')}</Text>
                </HStack>
              </button>
            ))}
          </VStack>
        ) : null}
        {error ? <ErrorText text={error} /> : null}
      </VStack>
    )
  }

  const preflight = selected.preflight
  const missing = preflight?.problems.find((problem) => problem.action.kind === 'download')
  return (
    <VStack gap={14}>
      <HStack gap={8}>
        <Button kind="secondary" onClick={() => setSelected(null)}>{tr('All templates')}</Button>
        <Text weight="medium" grow>{selected.name}</Text>
        <span className="comfy-badge">{tr(outputLabel(selected.output.type))}</span>
      </HStack>
      <Text size="caption" tone="secondary">{selected.description}</Text>
      {selected.license ? <Text size="caption" tone="secondary">{selected.license}</Text> : null}

      <CardSection title="Form">
        <VStack gap={14}>
          {selected.inputs.map((input) => (
            <Field
              key={input.key}
              input={input}
              value={values[input.key]}
              picked={picked[input.key]}
              busy={busy}
              onChange={(value) => setValues((current) => ({ ...current, [input.key]: value }))}
              onPick={() => void choose(input)}
            />
          ))}
        </VStack>
      </CardSection>

      {preflight ? (
        <VStack gap={8}>
          {preflight.problems.map((problem) => (
            <ProblemRow key={problem.code + problem.message} problem={problem} onAction={(kind, item) => void act(kind, item)} />
          ))}
          {preflight.warnings.map((warning) => (
            <ProblemRow key={warning.code + warning.message} problem={warning} onAction={() => {}} />
          ))}
          <HStack gap={10} wrap>
            {seconds(preflight.estimate.seconds) ? <Text size="caption" tone="secondary">{tr('About {time}').replace('{time}', seconds(preflight.estimate.seconds) ?? '')}</Text> : null}
            {preflight.estimate.vramGb ? <Text size="caption" tone="secondary">{tr('{gb} GB of graphics memory').replace('{gb}', String(preflight.estimate.vramGb))}</Text> : null}
            {preflight.estimate.gpu ? <Text size="caption" tone="secondary">{preflight.estimate.gpu}</Text> : null}
          </HStack>
        </VStack>
      ) : null}

      <HStack gap={8}>
        <Button disabled={busy || (preflight?.ok === false && !missing)} onClick={() => (missing ? void act('download', missing) : void run())}>
          {missing ? tr('Download model') : tr('Run')}
        </Button>
        <Button kind="secondary" disabled={busy} onClick={() => void open(selected.id)}>{tr('Check again')}</Button>
      </HStack>
      {note ? <Text size="caption" tone="secondary">{note}</Text> : null}
      {error ? <ErrorText text={error} /> : null}
    </VStack>
  )
}
