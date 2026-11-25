import type { CSSProperties } from 'react'
import { Button } from '../../../components/Controls'
import { Icon } from '../../../components/Icon'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import {
  NODE_ICON, nodeEditable, nodeTitle, type WorkflowHistoryStep, type WorkflowNode, type WorkflowTriggerState,
} from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'

const text = (node: WorkflowNode, key: string): string => {
  const value = node.fields[key]
  return typeof value === 'string' ? value.trim() : ''
}

/**
 * The compact body of a portable node on the canvas: enough to read what it does and, for a waiting
 * approval, the two buttons that answer it. The card itself keeps the shared status chip and port;
 * this only fills its middle, so an older routine/link/media card is unchanged.
 */
export function WorkflowNodeCardBody({ node, needsNewerApp, step, trigger, onDecide }: {
  node: WorkflowNode
  needsNewerApp: boolean
  step?: WorkflowHistoryStep | null
  trigger?: WorkflowTriggerState | null
  onDecide?: (approved: boolean) => void
}) {
  const tr = useTr()
  const editable = nodeEditable(node, needsNewerApp)
  const waiting = step?.waiting === true
  if (!editable) {
    return (
      <VStack gap={4} className="flow-workflow-body" data-readonly="true">
        <HStack gap={6} align="center">
          <Icon name="lock.locked" size={12} />
          <Text size="caption" weight="semibold">{tr('Read-only node')}</Text>
        </HStack>
        <Text size="caption" tone="secondary" lines={2}>
          {tr(nodeEditable(node) ? 'This workflow uses a newer format. Update Codync to change it.'
            : 'This step was added by a newer version of Codync.')}
        </Text>
        {needsNewerApp || !nodeEditable(node) ? (
          <span className="flow-newer-badge" title={tr('Needs a newer app')}>{tr('Needs newer app')}</span>
        ) : null}
      </VStack>
    )
  }
  const model = text(node, 'model') || text(node, 'provider')
  const detail = (() => {
    switch (node.kind) {
      case 'llm': return [model && `${tr('Model')}: ${model}`, text(node, 'prompt')].filter(Boolean).join(' · ')
      case 'condition': return [text(node, 'rule'), text(node, 'check')].filter(Boolean).join(' · ') || tr('Route by a check')
      case 'approval': return text(node, 'question') || tr('Wait for a decision')
      case 'output': return [text(node, 'target'), text(node, 'action')].filter(Boolean).join(' · ') || tr('Send the result')
      case 'asset': return text(node, 'fileId') || tr('A file from this computer')
      case 'trigger': return trigger ? [trigger.type, trigger.nextDueAt].filter(Boolean).join(' · ') : text(node, 'trigger')
      case 'image': return [model, text(node, 'aspect')].filter(Boolean).join(' · ') || tr('Picture or clip step')
      case 'text':
      case 'note': return text(node, 'text')
      case 'shape': return ''
      default: return text(node, 'agentId')
    }
  })()
  return (
    <VStack gap={5} className="flow-workflow-body" data-kind={node.kind}>
      {detail ? <Text size="caption" tone="secondary" lines={2}>{detail}</Text> : null}
      {node.kind === 'condition' ? (
        <HStack gap={6} className="flow-branch-legend" aria-label={tr('Yes and no outputs')}>
          <span className="flow-branch-tag" data-branch="yes"><Icon name="checkmark" size={9} />{tr('Yes')}</span>
          <span className="flow-branch-tag" data-branch="no"><Icon name="xmark" size={9} />{tr('No')}</span>
        </HStack>
      ) : null}
      {node.kind === 'asset' && typeof node.fields.fileId === 'string' && node.fields.fileId ? (
        <span className="flow-asset-chip"><Icon name="doc" size={11} />{node.fields.fileId}</span>
      ) : null}
      {node.kind === 'trigger' && trigger?.webhookUrl ? (
        <Text size="caption" tone="secondary" truncate title={trigger.webhookUrl}>{trigger.webhookUrl}</Text>
      ) : null}
      {node.kind === 'approval' && onDecide ? (
        <HStack gap={6} className="flow-approval-actions" onPointerDown={(event) => event.stopPropagation()}>
          <Button kind={waiting ? 'primary' : 'secondary'} disabled={!waiting} onClick={() => onDecide(true)}>{tr('Approve')}</Button>
          <Button kind="secondary" disabled={!waiting} onClick={() => onDecide(false)}>{tr('Reject')}</Button>
          {!waiting ? <Text size="caption" tone="secondary">{tr('Not waiting')}</Text> : null}
        </HStack>
      ) : null}
      {node.size?.font ? (
        <span className="flow-font-badge" style={{ '--flow-font': `${node.size.font}px` } as CSSProperties}>
          {tr('Font')}: {node.size.font}
        </span>
      ) : null}
    </VStack>
  )
}

/** The title of a portable node, with its kind's symbol for a quick scan. */
export function WorkflowNodeIcon({ node }: { node: WorkflowNode }) {
  const icon = nodeEditable(node) ? NODE_ICON[node.kind] ?? 'questionmark.circle' : 'lock.locked'
  return <Icon name={icon} size={24} />
}

export const workflowCardTitle = (node: WorkflowNode) => nodeTitle(node)
