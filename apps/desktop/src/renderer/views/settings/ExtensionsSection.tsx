import { useEffect, useMemo, useState } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'

interface ExtensionTool {
  name: string
  description: string
  inputSchema: unknown
}

interface Extension {
  id: string
  error: string | null
  manifest: {
    name: string
    version: string
    description: string
    runtime: string | null
    tools: ExtensionTool[]
  } | null
}

interface ExtensionsAnswer {
  runtime: string | null
  folder: string
  extensions: Extension[]
}

/**
 * The wasm extensions this computer has: a folder each with `extension.json` and a `.wasm` entry,
 * run by wasmtime or wasmer. Nothing here installs a runtime; the page says when one is missing.
 */
export function ExtensionsSection() {
  const app = useApp()
  const local = useModel(app.local)
  const running = local !== null && local.connection.kind === 'online'
  const [answer, setAnswer] = useState<ExtensionsAnswer | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const load = useMemo(() => async () => {
    if (!local) return
    setLoading(true)
    try {
      setAnswer(await (await local.ready()).call<ExtensionsAnswer>('extensions', {}))
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setLoading(false)
  }, [local])

  useEffect(() => {
    if (running) void load()
  }, [running, load])

  return (
    <CardSection
      title="Extensions"
      footer="Each folder holds an extension.json and its .wasm module. A tool runs only when something calls it; the module sees just the folders its manifest names."
      accessory={
        <Button kind="secondary" disabled={!running || loading} onClick={() => void load()}>
          Reload
        </Button>
      }
    >
      {!running ? (
        <Text size="caption" tone="secondary">Start this computerâ€™s host to see its extensions.</Text>
      ) : (
        <VStack gap={8}>
          <Text size="caption" tone="secondary">
            {answer?.runtime ? `Runtime: ${answer.runtime}` : 'No wasm runtime found. Install wasmtime or wasmer and add it to PATH.'}
          </Text>
          {answer?.folder ? <Text size="caption" tone="secondary">Folder: {answer.folder}</Text> : null}
          {answer?.extensions.length === 0 ? <Text size="caption" tone="secondary">No extensions installed yet.</Text> : null}
          {answer?.extensions.map((extension) => (
            <VStack key={extension.id} gap={3}>
              <Text weight="semibold">{extension.manifest?.name ?? extension.id}</Text>
              {extension.error ? (
                <Text size="caption" tone="danger">{extension.error}</Text>
              ) : (
                <>
                  <Text size="caption" tone="secondary">
                    {[extension.manifest?.version, extension.manifest?.description].filter(Boolean).join(' Â· ') || 'No description'}
                  </Text>
                  {extension.manifest?.tools.map((tool) => (
                    <Text key={tool.name} size="caption" tone="secondary">
                      {tool.name}{tool.description ? ` â€” ${tool.description}` : ''}
                    </Text>
                  ))}
                </>
              )}
            </VStack>
          ))}
          {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        </VStack>
      )}
    </CardSection>
  )
}

