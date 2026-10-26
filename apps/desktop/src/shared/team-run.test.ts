import assert from 'node:assert/strict'
import test from 'node:test'
import { MAX_TEAM_RUN_STAGES, teamRunIssue, teamRunMessage, type TeamRunDraft } from './team-run.ts'

const names: Record<string, string> = { a: 'Alice', b: 'Bob', c: 'Cy' }
const name = (id: string) => names[id] ?? 'Unknown'
const draft = (patch: Partial<TeamRunDraft> = {}): TeamRunDraft => ({
  mode: 'pipeline',
  input: '',
  stages: [{ botId: 'b', goal: 'Research the topic' }, { botId: 'c', goal: 'Write it up: {{previous}}' }],
  ...patch,
})

test('a complete plan has no issue', () => {
  assert.equal(teamRunIssue(draft(), 'a'), null)
  assert.equal(teamRunIssue(draft({ mode: 'parallel' }), 'a'), null)
})

test('the plan needs a coordinator, stages, bots and goals', () => {
  assert.match(teamRunIssue(draft(), '') ?? '', /coordinates/)
  assert.match(teamRunIssue(draft({ stages: [] }), 'a') ?? '', /at least one/)
  assert.match(teamRunIssue(draft({ stages: [{ botId: '', goal: 'x' }] }), 'a') ?? '', /Stage 1 needs a bot/)
  assert.match(teamRunIssue(draft({ stages: [{ botId: 'b', goal: '   ' }] }), 'a') ?? '', /Stage 1 needs a goal/)
})

test('the coordinator cannot be one of the stages', () => {
  assert.match(teamRunIssue(draft({ stages: [{ botId: 'a', goal: 'x' }] }), 'a') ?? '', /Stage 1.*itself/)
})

test('a bot can repeat in a pipeline but not in parallel', () => {
  const twice = [{ botId: 'b', goal: 'First' }, { botId: 'b', goal: 'Second' }]
  assert.equal(teamRunIssue(draft({ stages: twice }), 'a'), null)
  assert.match(teamRunIssue(draft({ mode: 'parallel', stages: twice }), 'a') ?? '', /Stage 2.*one task/)
})

test('the host’s limits are enforced before sending', () => {
  const many = Array.from({ length: MAX_TEAM_RUN_STAGES + 1 }, (_, i) => ({ botId: i % 2 ? 'b' : 'c', goal: 'x' }))
  assert.match(teamRunIssue(draft({ stages: many }), 'a') ?? '', /at most 8/)
  assert.match(teamRunIssue(draft({ stages: [{ botId: 'b', goal: 'é'.repeat(4_001) }] }), 'a') ?? '', /too long/)
  assert.equal(teamRunIssue(draft({ stages: [{ botId: 'b', goal: 'é'.repeat(4_000) }] }), 'a'), null)
})

test('a pipeline request names the tool, the order, the bots’ ids and the input', () => {
  const text = teamRunMessage(draft({ input: '  rust async  ' }), name)
  assert.match(text, /run_pipeline/)
  assert.match(text, /Input for stage 1 \(\{\{previous\}\}\): rust async/)
  assert.ok(text.indexOf('1. Bob (botId: b): Research the topic') < text.indexOf('2. Cy (botId: c): Write it up: {{previous}}'))
  assert.match(text, /tell me what came back/)
})

test('a pipeline without input leaves the input line out, and parallel names its own tool', () => {
  assert.doesNotMatch(teamRunMessage(draft(), name), /Input for stage 1/)
  const parallel = teamRunMessage(draft({ mode: 'parallel', input: 'ignored' }), name)
  assert.match(parallel, /delegate_batch/)
  assert.doesNotMatch(parallel, /Input for stage 1|run_pipeline/)
  assert.match(parallel, /Tasks:\n1\. Bob/)
})
