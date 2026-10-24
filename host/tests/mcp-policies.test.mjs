// Exercise the production schema and SQL transitions without compiling Rust.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DatabaseSync } from 'node:sqlite'

const schema = readFileSync(new URL('../src/store/mcp_policies.sql', import.meta.url), 'utf8')
const source = readFileSync(new URL('../src/store/mcp_policies.rs', import.meta.url), 'utf8')
function statement(method, operation) {
  const methodSource = source.slice(source.indexOf(`pub fn ${method}(`))
  const query = methodSource.match(new RegExp(`"(${operation}[^\"]+)"`))?.[1]
  assert.ok(query, `production SQL for ${method} must exist`)
  return query
}
const claim = statement('claim_mcp_call', 'UPDATE pending_tool_calls')
const decide = statement('decide_mcp_call', 'UPDATE pending_tool_calls')
const revoke = statement('set_mcp_policy', 'UPDATE pending_tool_calls')
const disconnect = statement('revoke_mcp_connector', 'UPDATE pending_tool_calls')
const forgetPolicy = statement('revoke_mcp_connector', 'DELETE FROM connector_tool_policies')
const readState = source.match(/"(SELECT bot_id,connector_id,tool_name,message,[^"]+)"/)?.[1]
assert.ok(readState, 'production request state query must exist')

function database() {
  const db = new DatabaseSync(':memory:')
  db.exec(schema)
  db.prepare('INSERT INTO pending_tool_calls VALUES(?,?,?,?,?,?,?,?)').run(
    'call', 'bot', 'connector', 'write', JSON.stringify({ arguments: { text: 'original' } }), 'pending', 1, 1000,
  )
  return db
}

test('production SQL blocks pending calls and grants one dispatch after approval', () => {
  const db = database()
  try {
    assert.equal(db.prepare(claim).run('call', 100).changes, 0)
    assert.equal(db.prepare(decide).run('call', 'approved', 100).changes, 1)
    assert.equal(db.prepare(claim).run('call', 100).changes, 1)
    assert.equal(db.prepare(claim).run('call', 100).changes, 0)
    assert.equal(JSON.parse(db.prepare('SELECT message FROM pending_tool_calls').get().message).arguments.text, 'original')
  } finally { db.close() }
})

test('declined and expired calls cannot be approved or dispatched', () => {
  for (const expired of [false, true]) {
    const db = database()
    try {
      if (!expired) db.prepare(decide).run('call', 'declined', 100)
      const now = expired ? 1001 : 100
      assert.equal(db.prepare(decide).run('call', 'approved', now).changes, 0)
      assert.equal(db.prepare(claim).run('call', now).changes, 0)
    } finally { db.close() }
  }
})

test('policy changes revoke approved calls and remain isolated to one connector/tool', () => {
  const db = database()
  try {
    db.prepare(decide).run('call', 'approved', 100)
    assert.equal(db.prepare(revoke).run('other', 'write').changes, 0)
    assert.equal(db.prepare(revoke).run('connector', 'other').changes, 0)
    assert.equal(db.prepare(revoke).run('connector', 'write').changes, 1)
    assert.equal(db.prepare(claim).run('call', 100).changes, 0)
  } finally { db.close() }
})

test('schema rejects unknown policy modes and duplicate policy keys', () => {
  const db = database()
  try {
    const insert = db.prepare('INSERT INTO connector_tool_policies VALUES(?,?,?)')
    assert.throws(() => insert.run('connector', 'write', 'unknown'))
    insert.run('connector', 'write', 'ask')
    assert.throws(() => insert.run('connector', 'write', 'allow'))
  } finally { db.close() }
})

test('disconnect and reinstall cannot resurrect approved calls or inherit allow rules', () => {
  const db = database()
  try {
    db.prepare('INSERT INTO connector_tool_policies VALUES(?,?,?)').run('connector', 'write', 'allow')
    db.prepare(decide).run('call', 'approved', 100)
    assert.equal(db.prepare(disconnect).run('connector', 'other-bot').changes, 0)
    assert.equal(db.prepare(disconnect).run('connector', 'bot').changes, 1)
    assert.equal(db.prepare(claim).run('call', 100).changes, 0)
    db.prepare(disconnect).run('connector', null)
    db.prepare(forgetPolicy).run('connector')
    assert.equal(db.prepare('SELECT COUNT(*) AS count FROM connector_tool_policies').get().count, 0)
  } finally { db.close() }
})

test('a claimed permission still expires before a delayed transport send', () => {
  const db = database()
  try {
    db.prepare(decide).run('call', 'approved', 100)
    db.prepare(claim).run('call', 100)
    const row = db.prepare(readState).get('call', 1001)
    assert.equal(Object.values(row).at(-1), 'expired')
    assert.equal(db.prepare(claim).run('call', 1001).changes, 0)
  } finally { db.close() }
})
