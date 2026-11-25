import { useEffect, useMemo, useState } from 'react'
import { CardSection, ChoicePicker, RowLabel, ValueRow } from '../../components/Controls'
import { Text } from '../../components/Text'
import { limit_note } from '../marketplace/mcp-limit-note'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import type { BotStore } from '../../store/bot-store'
import { McpToolsStore, type ToolMode } from '../../store/mcp-tools-store'

type ConnectorInfo = { id: string; name: string }

const MODES: { id: ToolMode; label: string }[] = [
  { id: 'ask', label: 'Ask first' },
  { id: 'allow', label: 'Allow' },
  { id: 'deny', label: 'Deny' },
]

/**
 * Settings › Tool policies: the tools the local computer has seen from each installed connector,
 * with the host's allow/ask/deny choice per tool. The choice is saved through the existing MCP
 * policy API (`McpToolsStore.setPolicy`), so the host stays the owner of authorization.
 */
export function McpToolPolicySection() {
  const app = useApp()
  const tr = useTr()
  const local = app.local
  const [connectors, setConnectors] = useState<ConnectorInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    if (!local) { setLoading(false); return }
    let live = true
    setLoading(true)
    void local.ready()
      .then((client) => client.call<{ items: ConnectorInfo[] }>('connectors'))
      .then((result) => { if (live) { setConnectors(result.items ?? []); setFailed(false) } })
      .catch(() => { if (live) setFailed(true) })
      .finally(() => { if (live) setLoading(false) })
    return () => { live = false }
  }, [local])

  if (!local) return <CardSection title="Tool policies">
    <Text size="caption" tone="secondary">{tr('Connect to this computer to manage tool policies.')}</Text>
  </CardSection>
  if (loading) return <CardSection title="Tool policies">
    <Text size="caption" tone="secondary">{tr('Loading tool policies…')}</Text>
  </CardSection>
  if (failed) return <CardSection title="Tool policies">
    <Text size="caption" tone="danger">{tr('Tool policies could not be loaded.')}</Text>
  </CardSection>
  if (!connectors.length) return <CardSection title="Tool policies">
    <Text size="caption" tone="secondary">{tr('No connectors are installed on this computer.')}</Text>
  </CardSection>
  return <>{connectors.map((connector) => <ConnectorPolicies key={connector.id} connector={connector} store={local} />)}</>
}

/** One connector's tools, read once when the section opens; no polling. */
function ConnectorPolicies({ connector, store }: { connector: ConnectorInfo; store: BotStore }) {
  const tr = useTr()
  const model = useModel(useMemo(() => new McpToolsStore(connector.id, () => store.ready()), [connector.id, store]))
  useEffect(() => {
    model.resume()
    void model.load()
    return () => model.dispose()
  }, [model])
  return (
    <CardSection
      title={connector.name}
      footer="Applies to every bot using this connection. Explicit choices override what the server reports."
    >
      <RowLabel label={tr('Working directory')} detail={limit_note(connector.name)} />
      {model.loading && !model.policies.length ? <Text size="caption" tone="secondary">{tr('Loading tools…')}</Text> : null}
      {!model.loading && !model.policies.length ? <Text size="caption" tone="secondary">{tr('No tools have been seen yet. Their policies appear after a tool runs.')}</Text> : null}
      {model.policies.map((policy) => (
        <ValueRow key={policy.tool} label={policy.tool} detail={policy.explicit === false ? tr('Server default') : undefined}>
          <ChoicePicker
            selection={policy.mode}
            options={MODES}
            onChange={(mode) => void model.setPolicy(policy.tool, mode)}
          />
        </ValueRow>
      ))}
      {model.busy ? <Text size="caption" tone="secondary">{tr('Saving…')}</Text> : null}
      {model.notice ? <Text size="caption" tone="secondary">{tr(model.notice)}</Text> : null}
      {model.error ? <Text size="caption" tone="danger">{model.error}</Text> : null}
    </CardSection>
  )
}
