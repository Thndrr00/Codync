import { useState } from 'react'
import type { ApprovalRule, RuleProposal } from '@shared/approval-rules'
import type { Entry, FileDiff, PermissionOption } from '@shared/models'
import { Button, Disclosure, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { Sheet } from '../../components/Overlay'
import { LocationSheet } from '../../components/LocationSheet'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import './permission-card.css'
import { useTr } from '../../lib/use-language'
import { AutoTextArea } from '../bots/parts'
import { trNow } from '../../lib/l10n.ts'
import { fill } from '../../lib/fill.ts'
import { approvalBrief } from './status-report.ts'


/** What the agent wants, from the tool's kind (also the notch island's headline). */
export const permissionHeadline = (toolKind?: string) => {
  switch (toolKind) {
    case 'execute':
      return trNow('Wants to run a command')
    case 'edit':
    case 'delete':
    case 'move':
      return trNow('Wants to change files')
    case 'fetch':
      return trNow('Wants to access the web')
    case 'read':
    case 'search':
      return trNow('Wants to read files')
    case 'question':
      return trNow('Asks you a question')
    case 'location':
      return trNow('Asks for your location')
    case 'plan':
      return trNow('Wants your approval of a plan')
    default:
      return trNow('Wants to use a tool')
  }
}

/**
 * Why a sensitive tool asks first, in plain words. A fixed map by tool name (never agent text), so a
 * person can tell what they're allowing without reading the command. Shown as a small info line.
 */
const TOOL_GUIDANCE: Record<string, string> = {
  execute: 'Runs a command on the computer. It can read, change or delete anything the bot can reach.',
  shell: 'Runs a command on the computer. It can read, change or delete anything the bot can reach.',
  bash: 'Runs a command on the computer. It can read, change or delete anything the bot can reach.',
  edit: 'Writes changes to a file on the computer.',
  write: 'Writes changes to a file on the computer.',
  file_write: 'Writes changes to a file on the computer.',
  delete: 'Deletes files or data on the computer.',
  move: 'Renames or moves files on the computer.',
  computer: 'Clicks and types on the computer’s screen.',
  computer_use: 'Clicks and types on the computer’s screen.',
  screen: 'Clicks and types on the computer’s screen.',
  message: 'Sends a message to someone outside this chat.',
  send: 'Sends a message to someone outside this chat.',
  external_message: 'Sends a message to someone outside this chat.',
  fetch: 'Reads a page from the internet.',
  web: 'Reads a page from the internet.',
}

/** The guidance line for a tool, matched by its kind or the card's title. */
export function toolGuidance(kind?: string, title?: string): string | null {
  const byKind = kind ? TOOL_GUIDANCE[kind.toLowerCase()] : undefined
  if (byKind) return byKind
  return title ? TOOL_GUIDANCE[title.toLowerCase()] ?? null : null
}

/** The host puts this note and one agent name per line in a deletion approval's details. */
const deletionNote = 'Only the owner can approve this deletion.'
const isDeletionApproval = (d: Entry['data']) =>
  !!d.detail?.includes(deletionNote) && ['Delete agents', 'Delete an agent'].includes(d.title ?? '')
const deletionAgents = (d: Entry['data']) =>
  (d.detail ?? '').split('\n\n').slice(1).join('\n\n').split('\n').map((name) => name.trim()).filter(Boolean)

const rank: Record<string, number> = { allow_once: 0, allow_always: 1, reject_once: 2, reject_always: 3 }

/** Allow before deny, once before always. */
export const sortedOptions = (options: PermissionOption[]) => [...options].sort((a, b) => (rank[a.kind] ?? 9) - (rank[b.kind] ?? 9))

export function optionLabel(o: PermissionOption) {
  switch (o.kind) {
    case 'allow_once':
      return trNow('Allow once')
    case 'allow_always':
      return trNow('Always allow')
    case 'reject_once':
      return 'Deny'
    case 'reject_always':
      return trNow('Never')
    default:
      return o.name
  }
}

/** Approval card in Grok Bot's choice-card style: what the agent wants, where it runs, the answers. */
export function PermissionCard({ entry, hostName, answering, respond, propose, otherQuestions = 0 }: {
  entry: Entry
  hostName: string
  answering: string | null
  respond: (option: string | null, rule?: ApprovalRule) => void
  /** Asks the host which rule "always allow" would keep; without it the answer goes straight to the agent. */
  propose?: () => Promise<RuleProposal | null>
  /** Other question cards still open in this chat; one question answering never closes another. */
  otherQuestions?: number
}) {
  const tr = useTr()
  const [expanded, setExpanded] = useState(false)
  const [rejectedShake, setRejectedShake] = useState(false)
  const [locationOpen, setLocationOpen] = useState(false)
  const [changeText, setChangeText] = useState('')
  const [always, setAlways] = useState<{ option: PermissionOption; proposal: RuleProposal; pattern: string } | null>(null)
  const d = entry.data
  const guidance = toolGuidance(d.toolKind, d.title)
  const pending = d.status === 'pending'
  const location = d.toolKind === 'location'
  // An expired location card still answers (OpenBot nightly v0.77.0's CAPTCHA-card fix).
  const answerable = location ? pending || d.status === 'expired' : pending
  const deletion = isDeletionApproval(d)
  // The agent's scope, risks and recommendation sit in this card, not behind a Details step.
  const brief = approvalBrief(d.detail)
  const detailText = brief?.rest ?? d.detail ?? ''
  const hasDetail = !!d.command || !!detailText || !!d.diffs?.length
  const options = sortedOptions(d.options ?? [])
  const once = options.find((o) => o.kind === 'allow_once')
  // "Always" shows the rule it would keep, which the person may edit, before anything is answered.
  const choose = async (o: PermissionOption) => {
    if (o.kind.startsWith('reject')) {
      setRejectedShake(true)
      setTimeout(() => setRejectedShake(false), 450)
    }
    if (o.kind === 'allow_always' && propose) {
      const proposal = await propose()
      if (proposal) return setAlways({ option: o, proposal, pattern: proposal.rule?.pattern ?? '' })
    }
    respond(o.optionId)
  }
  const cwd = d.cwd ? ` · ${d.cwd.split('/').filter(Boolean).pop() ?? d.cwd}` : ''
  return (
    <div className="agent-inset">
      <VStack gap={10} className={`permission-card t-input-shake ${rejectedShake ? "is-invalid" : ""}`}>
        <HStack gap={8}>
          <Text size="headline" tone="text">{deletion ? tr('Wants to delete agents') : permissionHeadline(d.toolKind)}</Text>
          {answerable ? <span className="permission-card-dot" /> : null}
        </HStack>
        <Text as="div" size="subheadline" mono tone="secondary" lines={expanded ? undefined : 3} className="selectable permission-card-title">
          {d.title ?? ''}
        </Text>
        <HStack gap={6}>
          <Icon name="desktopcomputer" size={10} color="var(--tertiary)" />
          <Text size="caption" tone="tertiary">{fill(tr('Runs on {name}{path}'), { name: hostName, path: cwd })}</Text>
        </HStack>
        {guidance ? (
          <HStack gap={6} className="permission-guidance">
            <Icon name="info.circle" size={10} color="var(--secondary)" />
            <Text size="caption" tone="secondary" title={tr(guidance)}>{tr(guidance)}</Text>
          </HStack>
        ) : null}
        {d.toolKind === 'question' && answerable && otherQuestions > 0 ? (
          <Text size="caption" tone="secondary">
            {otherQuestions === 1 ? tr('Another question is still open') : fill(tr('{n} other questions are still open'), { n: String(otherQuestions) })}
          </Text>
        ) : null}
        {brief ? (
          <VStack gap={6} className="permission-brief">
            <BriefRow label="Scope" value={brief.scope} />
            <BriefRow label="Risks" value={brief.risks} />
            <BriefRow label="Recommendation" value={brief.recommendation} />
          </VStack>
        ) : null}
        {deletion ? (
          <VStack gap={4}>
            <Text size="caption" tone="secondary">{tr(deletionNote)}</Text>
            {deletionAgents(d).map((name) => (
              <Text key={name} size="caption" mono tone="text">{name}</Text>
            ))}
          </VStack>
        ) : hasDetail ? (
          <Disclosure expanded={expanded} onToggle={() => setExpanded((e) => !e)} label={<Text size="caption" weight="medium" tone="secondary">{tr(`Details`)}</Text>}>
            <VStack gap={8} className={`permission-card-details t-resize ${expanded ? "is-open" : ""}`}>
              {d.command ? <CodeBox text={d.command} /> : null}
              {detailText ? <CodeBox text={detailText} /> : null}
              {(d.diffs ?? []).map((diff, i) => <DiffView key={i} diff={diff} />)}
            </VStack>
          </Disclosure>
        ) : null}
        {location ? (
          answerable ? (
            <VStack gap={8}>
              <Text size="caption" tone="secondary">
                {tr(d.status === 'expired' ? 'The agent moved on. Sharing still sends it as your next message.' : 'Nothing is sent until you share it.')}
              </Text>
              <Button kind="primary" disabled={answering !== null} onClick={() => setLocationOpen(true)}>{tr('Share location')}</Button>
            </VStack>
          ) : (
            <Text as="div" size="caption" weight="medium" tone="secondary">{outcome(entry)}</Text>
          )
        ) : pending ? (
          <>
            <div className="permission-card-options">
              {options.map((o, i) => (
                <div key={o.optionId}>
                  {i > 0 ? <div className="thread-hairline" /> : null}
                  <button
                    className={answering === null || answering === o.optionId ? 'permission-option' : 'permission-option dimmed'}
                    disabled={answering !== null}
                    onClick={() => void choose(o)}
                  >
                    <Text size="body" weight={o.kind === 'allow_once' ? 'semibold' : 'regular'} tone={o.kind.startsWith('allow') || o.kind === 'stop' ? 'text' : 'danger'} className="permission-option-label">{optionLabel(o)}</Text>
                    {answering === o.optionId ? <Spinner size={14} /> : null}
                  </button>
                </div>
              ))}
            </div>
            {d.toolKind === 'plan' ? (
              <VStack gap={8}>
                <Text size="caption" tone="secondary">{tr('What should change?')}</Text>
                <AutoTextArea
                  className="field-box"
                  value={changeText}
                  minRows={1}
                  maxRows={4}
                  placeholder={tr('Tell the agent what to change')}
                  onChange={setChangeText}
                />
                <Button
                  kind="secondary"
                  disabled={answering !== null || !changeText.trim()}
                  onClick={() => respond(`change:${changeText.trim()}`)}
                >
                  {tr('Send change')}
                </Button>
              </VStack>
            ) : null}
          </>
        ) : (
          <Text as="div" size="caption" weight="medium" tone="secondary">{outcome(entry)}</Text>
        )}
        {pending && always ? (
          <VStack gap={8} className="permission-always">
            {always.proposal.rule ? (
              <>
                <Text size="footnote" tone="secondary">{tr('Always allow keeps this rule for the bot. Edit it to cover more: * stands for any text.')}</Text>
                <AutoTextArea className="field-box" value={always.pattern} minRows={1} maxRows={3} onChange={(pattern) => setAlways({ ...always, pattern })} />
                <HStack gap={8}>
                  <Button
                    kind="primary"
                    disabled={!always.pattern.trim()}
                    onClick={() => respond(always.option.optionId, { ...always.proposal.rule!, pattern: always.pattern.trim() })}
                  >
                    {tr('Keep rule and allow')}
                  </Button>
                  <Button kind="secondary" onClick={() => setAlways(null)}>{tr('Cancel')}</Button>
                </HStack>
              </>
            ) : (
              <>
                <Text size="footnote" tone="secondary">{always.proposal.reason ?? tr('No rule can be kept for this request.')}</Text>
                <HStack gap={8}>
                  {once ? <Button kind="primary" onClick={() => respond(once.optionId)}>{tr('Allow once')}</Button> : null}
                  <Button kind="secondary" onClick={() => setAlways(null)}>{tr('Cancel')}</Button>
                </HStack>
              </>
            )}
          </VStack>
        ) : null}
      </VStack>
      <Sheet open={locationOpen} onClose={() => setLocationOpen(false)} width={420}>
        <LocationSheet entry={entry} close={() => setLocationOpen(false)} />
      </Sheet>
    </div>
  )
}

/** One labelled part of the agent's approval brief, shown without a disclosure. */
function BriefRow({ label, value }: { label: string; value: string }) {
  const tr = useTr()
  if (!value) return null
  return (
    <VStack gap={2}>
      <Text size="caption" weight="semibold" tone="secondary">{tr(label)}</Text>
      <Text size="callout" tone="text">{value}</Text>
    </VStack>
  )
}

function outcome(entry: Entry) {
  const d = entry.data
  if (d.toolKind === 'location') return d.status === 'answered' ? trNow('Location shared') : trNow('Expired — you can still share')
  switch (d.status) {
    case 'answered': {
      const chosen = d.options?.find((o) => o.optionId === d.selected)
      switch (chosen?.kind) {
        case 'allow_once':
          return trNow('Allowed once')
        case 'allow_always':
          return trNow('Always allowed')
        case 'reject_once':
          return trNow('Denied')
        case 'reject_always':
          return trNow('Never allowed')
        default:
          return chosen?.name ?? trNow('Answered')
      }
    }
    case 'cancelled':
      return 'Cancelled'
    default:
      return trNow('Expired — the agent moved on')
  }
}

export function CodeBox({ text }: { text: string }) {
  return (
    <div className="code-box">
      <pre className="selectable code-box-text">{text}</pre>
    </div>
  )
}

export function DiffView({ diff }: { diff: FileDiff }) {
  const lines = diff.patch.split('\n').slice(0, 80)
  return (
    <VStack gap={4}>
      <HStack gap={6} className="diff-header">
        <Icon name={diff.isNew ? 'doc.badge.plus' : 'doc.text'} size={10} />
        <Text truncate grow>{diff.path.split('/').pop()}</Text>
        <Text tone="added">+{diff.added}</Text>
        <Text tone="removed">−{diff.removed}</Text>
      </HStack>
      <div className="code-box diff-box">
        <div className="code-box-text">
          {lines.map((line, i) => (
            <Text as="div" key={i} tone={line.startsWith('+') ? 'added' : line.startsWith('-') ? 'removed' : 'secondary'} className="diff-line">
              {line || ' '}
            </Text>
          ))}
        </div>
      </div>
    </VStack>
  )
}
