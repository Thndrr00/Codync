import {test} from 'node:test'
import assert from 'node:assert/strict'
import {readFileSync,mkdtempSync,rmSync,rmdirSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {DatabaseSync} from 'node:sqlite'
const schema=readFileSync(new URL('../src/store/analytics.sql',import.meta.url),'utf8')
const source=readFileSync(new URL('../src/store/analytics.rs',import.meta.url),'utf8')
const sql=(prefix)=>{const found=[...source.matchAll(/"([^"\n]*(?:SELECT|INSERT|DELETE)[^"\n]*)"/g)].map(match=>match[1]).find(statement=>statement.startsWith(prefix));assert.ok(found,prefix);return found}
function open(path=':memory:'){const db=new DatabaseSync(path);db.exec('CREATE TABLE IF NOT EXISTS kv(k TEXT PRIMARY KEY,v TEXT NOT NULL);'+schema);return db}
function enqueue(db,id,client=false){
  db.exec('BEGIN')
  try {
    if(!Object.values(db.prepare(sql('SELECT COALESCE')).get())[0]){db.exec('ROLLBACK');return false}
    if(client&&!db.prepare(sql('INSERT INTO analytics_client_events')).run(id,Date.now()).changes){db.exec('ROLLBACK');return false}
    const added=db.prepare(sql('INSERT INTO analytics_outbox')).run(id,JSON.stringify({uuid:id,event:'fixture'})).changes
    db.exec(sql('DELETE FROM analytics_outbox WHERE seq'))
    db.exec(sql('DELETE FROM analytics_client_events WHERE id'))
    db.exec('COMMIT');return!!added
  } catch(error){db.exec('ROLLBACK');throw error}
}
test('durable analytics events survive reopening with their original identities',()=>{
  const folder=mkdtempSync(join(tmpdir(),'codync-analytics-'))
  const path=join(folder,'events.sqlite');let db=open(path)
  try {
    enqueue(db,'id-a',true);db.close();db=open(path)
    assert.equal(JSON.parse(db.prepare(sql('SELECT payload')).get().payload).uuid,'id-a')
    db.prepare(sql('DELETE FROM analytics_outbox WHERE id')).run('id-a')
    assert.equal(enqueue(db,'id-a',true),false)
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM analytics_outbox').get().n,0)
  } finally {db.close();rmSync(path);rmdirSync(folder)}
})
test('acknowledging one snapshot cannot remove events queued while it was in flight',()=>{
  const db=open()
  try {
    enqueue(db,'first');enqueue(db,'later')
    db.prepare(sql('DELETE FROM analytics_outbox WHERE id')).run('first')
    assert.equal(JSON.parse(db.prepare(sql('SELECT payload')).get().payload).uuid,'later')
  } finally{db.close()}
})
test('opt-out drops persisted events and prevents subsequent enqueue in the same consent state',()=>{
  const db=open()
  try {
    enqueue(db,'first');db.prepare(sql("INSERT INTO kv(k,v) VALUES('analytics_enabled'")).run('0');db.exec('DELETE FROM analytics_outbox')
    assert.equal(enqueue(db,'disabled',true),false)
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM analytics_outbox').get().n,0)
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM analytics_client_events').get().n,0)
  } finally{db.close()}
})
test('queue bounds retain the newest 500 events and failed writes do not claim client event identities',()=>{
  const db=open()
  try {
    for(let n=0;n<505;n++)enqueue(db,`id-${n}`)
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM analytics_outbox').get().n,500)
    assert.equal(JSON.parse(db.prepare(sql('SELECT payload')).get().payload).uuid,'id-5')
    db.exec("CREATE TRIGGER reject_event BEFORE INSERT ON analytics_outbox BEGIN SELECT RAISE(ABORT,'fixture failure'); END")
    assert.throws(()=>enqueue(db,'failed',true),/fixture failure/)
    assert.equal(db.prepare("SELECT COUNT(*) AS n FROM analytics_client_events WHERE id='failed'").get().n,0)
  }finally{db.close()}
})
