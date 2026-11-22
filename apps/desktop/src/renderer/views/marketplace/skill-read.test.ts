import assert from 'node:assert/strict'
import { test } from 'node:test'
import { skillDetailsTarget } from './skill-read.ts'

test('the skill preview reads the installed copy the agent uses', () => {
  assert.deepEqual(skillDetailsTarget({ id: 'local-skill' }, []), { id: 'local-skill' })
  assert.deepEqual(skillDetailsTarget({ source: 'openbot/digest', installed: false }, []), { source: 'openbot/digest' })
  assert.deepEqual(
    skillDetailsTarget({ source: 'openbot/digest', installed: true }, [{ id: 'digest-2', source: 'openbot/digest' }]),
    { id: 'digest-2' },
    'a marketplace skill that changed its own file shows that file',
  )
  assert.deepEqual(skillDetailsTarget({ source: 'OpenBot/Digest', installed: true }, []), { id: 'openbot/digest' })
})
