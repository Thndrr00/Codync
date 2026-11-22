import assert from 'node:assert/strict'
import { test } from 'node:test'
import { approvalBrief, briefStatus } from './status-report.ts'

test('the status line leads with the result and leaves out logs, ids and versions', () => {
  assert.equal(briefStatus('Published the draft. Now visible to users.'), 'Published the draft. Now visible to users.')
  assert.equal(
    briefStatus('[trace] 4f9a2c1b\nSkipped the review step: the checker was busy.\nid=run-8812\nRunning v2.47.0'),
    'Skipped the review step: the checker was busy.',
  )
  assert.equal(briefStatus('Running v2.47.0'), 'Running')
  assert.equal(briefStatus('  [debug] reading config  '), '')
  assert.equal(briefStatus(null), '')
})

test('an approval brief reads scope, risks and recommendation from one detail', () => {
  const brief = approvalBrief([
    'Publish the release note.',
    '',
    'Scope: the Drafts folder and the public site',
    'Risks: a half-written draft could go out',
    'Recommendation: wait for the check, then publish',
  ].join('\n'))
  assert.deepEqual(brief, {
    scope: 'the Drafts folder and the public site',
    risks: 'a half-written draft could go out',
    recommendation: 'wait for the check, then publish',
    rest: 'Publish the release note.',
  })
})

test('an ordinary detail is not reshaped into a partial brief', () => {
  assert.equal(approvalBrief('Runs a command on the computer.'), null)
  assert.equal(approvalBrief('Scope: only this folder\nSome notes'), null)
  assert.deepEqual(approvalBrief('Risk: one file\nRecommendation: allow once'), {
    scope: '',
    risks: 'one file',
    recommendation: 'allow once',
    rest: '',
  })
})
