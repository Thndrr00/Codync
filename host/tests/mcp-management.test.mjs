// Execute the management module's actual SQL without compiling Rust.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DatabaseSync } from 'node:sqlite'

const schema = readFileSync(new URL('../src/store/mcp_policies.sql', import.meta.url), 'utf8')
const source = readFileSync(new URL('../src/store/mcp_policies/management.rs', import.meta.url), 'utf8')
function sql(method, prefix) {
  const query = source.slice(source.indexOf(`pub fn ${method}(`)).match(new RegExp(`"(${prefix}[^\"]+)"`))?.[1]
  assert.ok(query, `production query for ${method}: ${prefix}`)
  return query
}
const policies = sql('mcp_tool_settings', 'SELECT names.tool_name')
const inbox = sql('list_mcp_calls', 'SELECT id,bot_id')
const ids = source.match(/"(SELECT id FROM pending_tool_calls[^\"]+)"/)?.[1]
assert.ok(ids, 'production revocation selection query exists')
const revoke = sql('revoke_mcp_calls', 'UPDATE pending_tool_calls')
const reset = sql('revoke_mcp_calls', 'DELETE FROM connector_tool_policies')
function database() { const db = new DatabaseSync(':memory:'); db.exec(schema); return db }
function request(db, id, connector = 'one', tool = 'write', status = 'pending', expires = 1000) {
  db.prepare('INSERT INTO pending_tool_calls VALUES(?,?,?,?,?,?,?,?)').run(id, 'bot', connector, tool,
    JSON.stringify({ jsonrpc: '2.0', id, method: 'tools/call', params: { name: tool, arguments: { text: 'exact original' } } }), status, 1, expires)
}

test('settings return only the selected connector overrides with exact tool names', () => {
  const db = database()
  try {
    for (const row of [['one', 'Write', 'allow'], ['one', 'write', 'deny'], ['other', 'write', 'allow']]) {
      db.prepare('INSERT INTO connector_tool_policies VALUES(?,?,?)').run(...row)
    }
    assert.deepEqual(db.prepare(policies).all('one').map((row) => [row.tool_name, row.mode]), [['Write', 'allow'], ['write', 'deny']])
    assert.equal(db.prepare(policies).all('missing').length, 0)
  } finally { db.close() }
})

test('the inbox is paginated and excludes other connectors, expired calls and terminal decisions', () => {
  const db = database()
  try {
    for (let i = 0; i < 75; i++) request(db, String(i).padStart(3, '0'))
    request(db, 'foreign', 'other'); request(db, 'expired', 'one', 'write', 'pending', 100)
    request(db, 'declined', 'one', 'write', 'declined'); request(db, 'revoked', 'one', 'write', 'revoked')
    const first = db.prepare(inbox).all('one', '', 100)
    assert.equal(first.length, 51)
    const second = db.prepare(inbox).all('one', first[49].id, 100)
    assert.equal(second.length, 25)
    assert.equal(new Set([...first.slice(0, 50), ...second].map((row) => row.id)).size, 75)
  } finally { db.close() }
})

test('restoring a default atomically removes one override and revokes only that tool authorizations', () => {
  const db = database()
  try {
    db.prepare('INSERT INTO connector_tool_policies VALUES(?,?,?)').run('one', 'write', 'allow')
    db.prepare('INSERT INTO connector_tool_policies VALUES(?,?,?)').run('one', 'read', 'deny')
    request(db, 'waiting'); request(db, 'approved', 'one', 'write', 'approved')
    request(db, 'reserved', 'one', 'write', 'dispatched'); request(db, 'read', 'one', 'read')
    request(db, 'foreign', 'other')
    db.exec('BEGIN')
    assert.equal(db.prepare(ids).all('one', 'write').length, 3)
    assert.equal(db.prepare(revoke).run('one', 'write').changes, 3)
    db.prepare(reset).run('one', 'write')
    db.exec('COMMIT')
    assert.deepEqual(db.prepare(policies).all('one').map((row) => row.tool_name), ['read'])
    assert.deepEqual(db.prepare(inbox).all('one', '', 100).map((row) => row.id), ['read'])
    assert.equal(db.prepare(inbox).all('other', '', 100).length, 1)
  } finally { db.close() }
})

test('bulk revocation covers every bot and tool on one connector without deleting policies or payloads', () => {
  const db = database()
  try {
    request(db, 'one'); request(db, 'two', 'one', 'read', 'approved'); request(db, 'reserved', 'one', 'write', 'dispatched')
    request(db, 'foreign', 'other')
    db.prepare("UPDATE pending_tool_calls SET bot_id='second-bot' WHERE id='two'").run()
    db.prepare('INSERT INTO connector_tool_policies VALUES(?,?,?)').run('one', 'read', 'allow')
    assert.equal(db.prepare(revoke).run('one', null).changes, 3)
    assert.equal(db.prepare(revoke).run('one', null).changes, 0)
    assert.equal(db.prepare(policies).all('one')[0].mode, 'allow')
    const saved = JSON.parse(db.prepare("SELECT message FROM pending_tool_calls WHERE id='one'").get().message)
    assert.equal(saved.params.arguments.text, 'exact original')
    assert.equal(db.prepare(inbox).all('other', '', 100).length, 1)
  } finally { db.close() }
})
