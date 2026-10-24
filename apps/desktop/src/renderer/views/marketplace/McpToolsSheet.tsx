import { useEffect, useMemo, useState } from 'react'
import { Button, CardForm, CardSection, SegmentedChoice } from '../../components/Controls'
import { Dialog, ModalHeader, SheetPage } from '../../components/Overlay'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useStore } from '../../store/context'
import { McpToolsStore, type ToolMode } from '../../store/mcp-tools-store'
import { ErrorText, Field } from './pieces'
import './mcp-tools.css'

const MODES: { id: ToolMode; label: string }[] = [
  { id: 'ask', label: 'Ask first' }, { id: 'allow', label: 'Allow' }, { id: 'deny', label: 'Deny' },
]

export function McpToolsSheet({ connector }: { connector: { id: string; name: string } }) {
  const store = useStore()
  const model = useModel(useMemo(() => new McpToolsStore(connector.id, () => store.ready()), [store, connector.id]))
  const [tool, setTool] = useState('')
  const [mode, setMode] = useState<ToolMode>('ask')
  const [receiptId, setReceiptId] = useState('')
  const [revoking, setRevoking] = useState<McpToolsStore | null>(null)
  useEffect(() => {
    setTool('')
    setMode('ask')
    setReceiptId('')
    setRevoking(null)
    model.resume()
    void model.load()
    const timer = setInterval(() => void model.load(), 5_000)
    return () => { clearInterval(timer); model.dispose() }
  }, [model])
  const detail = model.detail
  const pending = detail?.status === 'pending' && detail.expiresAt > Date.now()
  return <SheetPage>
    <ModalHeader title={`${connector.name} tools`} />
    <CardForm>
      <CardSection title="Tool policies" footer="Applies to every bot using this connection. Observed read-only tools may run automatically; unknown tools require Ask first. Explicit policies override server hints.">
        <Field label="Exact tool name" value={tool} onChange={setTool} placeholder="Tool name from an approval card" />
        <SegmentedChoice selection={mode} onChange={setMode} options={MODES} />
        <Button disabled={model.busy || !tool.trim()} onClick={() => void model.setPolicy(tool.trim(), mode)}>Save policy</Button>
        {model.policies.map((policy) => <VStack key={policy.tool} gap={6}>
          <Text weight="semibold">{policy.tool}</Text>
          <Text tone="secondary">{MODES.find((item) => item.id === policy.mode)?.label} · {policy.explicit === false ? 'Server default' : 'Owner override'}</Text>
          <Button kind="secondary" disabled={model.busy} onClick={() => { setTool(policy.tool); setMode(policy.mode) }}>Edit policy</Button>
          <Button kind="secondary" disabled={model.busy} onClick={() => void model.resetPolicy(policy.tool)}>Restore server default</Button>
        </VStack>)}
        {!model.loading && !model.policies.length && <Text tone="secondary">No overrides. Observed read-only tools use Allow; unknown tools use Ask first.</Text>}
      </CardSection>
      <CardSection title="Current requests" footer="Includes waiting approvals and reserved dispatches. Revocation cannot undo an effect already sent to the server; it does not disable future requests.">
        <Button kind="secondary" disabled={model.loading || model.busy} onClick={() => void model.load()}>Refresh</Button>
        {model.loading && <Text as="div" role="status">Loading requests…</Text>}
        {!model.loading && !model.requests.length && <Text tone="secondary">No current requests.</Text>}
        {model.requests.map((request) => <VStack key={request.id} gap={4}>
          <Text weight="semibold">{request.tool}</Text>
          <Text mono size="caption">Request ID: {request.id}</Text>
          <Text tone="secondary">{store.bots.get(request.botId)?.name ?? request.botId} · {request.status} · Expires {new Date(request.expiresAt).toLocaleTimeString()}</Text>
          <Button kind="secondary" disabled={model.busy} onClick={() => void model.inspect(request)}>Review exact arguments</Button>
        </VStack>)}
        {model.next && <Button kind="secondary" disabled={model.loading || model.busy} onClick={() => void model.load(true)}>Load more</Button>}
        <Button kind="secondary" disabled={model.busy} onClick={() => setRevoking(model)}>Revoke current requests</Button>
      </CardSection>
      <CardSection title="Recover saved result" footer="Retrieval only reads a receipt. It never authorizes or executes a tool again. Keep the request ID from its card; a missing receipt means the execution outcome may be unknown.">
        <Field label="Saved request ID" value={receiptId} onChange={setReceiptId} placeholder="Request ID" />
        <Button kind="secondary" disabled={!receiptId.trim() || model.inspecting} onClick={() => void model.retrieveResult(receiptId.trim())}>Retrieve result without running</Button>
        {model.receipt && <><Text mono size="caption">Request ID: {model.receipt.id}</Text><Text>Status: {model.receipt.status}</Text>{model.receipt.reply ? <pre className="mcp-tool-arguments">{JSON.stringify(model.receipt.reply, null, 2)}</pre> : <Text tone="secondary">No receipt saved. Inspect the upstream state before retrying.</Text>}</>}
      </CardSection>
      {model.inspecting && <CardSection><Text as="div" role="status">Loading request details…</Text></CardSection>}
      {detail && <CardSection title={detail.tool} footer={`Bot: ${detail.botId} · ${detail.status}`}>
        <pre className="mcp-tool-arguments">{JSON.stringify(detail.arguments, null, 2)}</pre>
        {pending && <>
          <Button disabled={model.busy} onClick={() => void model.decide(detail.id, true)}>Approve &amp; run</Button>
          <Button kind="secondary" disabled={model.busy} onClick={() => void model.decide(detail.id, false)}>Decline</Button>
        </>}
        <Button kind="secondary" onClick={() => model.closeDetail()}>Close request</Button>
      </CardSection>}
      {model.notice && <CardSection><Text as="div" role="status">{model.notice}</Text></CardSection>}
      {model.error && <CardSection><ErrorText text={model.error} /></CardSection>}
    </CardForm>
    <Dialog open={revoking === model} title="Revoke current requests?" message={`Revoke waiting and reserved requests for ${connector.name} across all bots. Effects already sent cannot be undone. Future requests keep their configured policies.`}
      onClose={() => setRevoking(null)} actions={[{ title: 'Revoke requests', destructive: true, action: () => { if (revoking === model) void revoking.revokeAll() } }]} />
  </SheetPage>
}
