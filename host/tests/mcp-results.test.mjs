import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, mkdtempSync, rmSync, rmdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { DatabaseSync } from 'node:sqlite'
const schema = readFileSync(new URL('../src/store/mcp_policies.sql', import.meta.url), 'utf8')
const source = readFileSync(new URL('../src/store/mcp_policies/results.rs', import.meta.url), 'utf8')
const management = readFileSync(new URL('../src/store/mcp_policies/management.rs', import.meta.url), 'utf8')
function query(text, prefix) { const result = text.match(new RegExp(`"(${prefix}[^\"]+)"`))?.[1]; assert.ok(result); return result }
const observe = query(source, 'INSERT INTO connector_tool_catalog')
const policies = query(management, 'SELECT names.tool_name')
const record = query(source, 'INSERT INTO mcp_call_results')
const read = query(source, 'SELECT result FROM mcp_call_results')
const execution = query(source, 'INSERT INTO mcp_execution_claims')
test('deleting a request removes its sensitive receipt and execution claim without foreign-key enforcement',()=>{
  const db=new DatabaseSync(':memory:');db.exec(schema)
  try {
    db.prepare('INSERT INTO pending_tool_calls VALUES(?,?,?,?,?,?,?,?)').run('call','bot','connector','write','{}','dispatched',1,999)
    db.prepare(record).run('call','{"result":"private"}',1)
    db.prepare(execution).run('call',1)
    db.prepare('DELETE FROM pending_tool_calls WHERE id=?').run('call')
    assert.equal(db.prepare(read).get('call'),undefined)
    assert.equal(db.prepare('SELECT * FROM mcp_execution_claims WHERE call_id=?').get('call'),undefined)
  }finally{db.close()}
})

test('provider execution has a durable single attempt independent of result delivery', () => {
  const db = new DatabaseSync(':memory:'); db.exec(schema)
  try {
    db.prepare(execution).run('call',1)
    assert.throws(()=>db.prepare(execution).run('call',2))
    assert.equal(db.prepare(read).get('call'),undefined)
    assert.doesNotThrow(()=>db.prepare(execution).run('other-call',3))
  } finally { db.close() }
})

test('observed read-only defaults are visible while exact owner policies override them', () => {
  const db = new DatabaseSync(':memory:'); db.exec(schema)
  try {
    db.prepare(observe).run('connector', 'read', 1)
    db.prepare(observe).run('connector', 'write', 0)
    assert.deepEqual(db.prepare(policies).all('connector').map((row) => [row.tool_name, row.mode]), [['read', 'allow'], ['write', 'ask']])
    db.prepare('INSERT INTO connector_tool_policies VALUES(?,?,?)').run('connector', 'read', 'ask')
    assert.equal(db.prepare(policies).all('connector')[0].mode, 'ask')
    db.prepare('DELETE FROM connector_tool_policies WHERE connector_id=? AND tool_name=?').run('connector', 'read')
    assert.equal(db.prepare(policies).all('connector')[0].mode, 'allow')
    assert.equal(db.prepare(policies).all('other').length, 0)
  } finally { db.close() }
})
test('execution receipts survive reopening and the database refuses overwriting an existing identity', () => {
  const directory = mkdtempSync(join(tmpdir(), 'codync-mcp-result-'))
  const file = join(directory, 'receipts.sqlite')
  let db = new DatabaseSync(file)
  const reply = JSON.stringify({ jsonrpc: '2.0', id: 'rpc', result: { content: [{ type: 'text', text: 'original result' }] } })
  try {
    db.exec(schema)
    db.prepare(record).run('call-id', reply, 1)
    db.close(); db = new DatabaseSync(file)
    assert.equal(db.prepare(read).get('call-id').result, reply)
    assert.throws(() => db.prepare(record).run('call-id', '{}', 2))
    assert.equal(db.prepare(read).get('call-id').result, reply)
    assert.equal(db.prepare(read).get('unknown'), undefined)
  } finally { db.close(); rmSync(file); rmdirSync(directory) }
})
