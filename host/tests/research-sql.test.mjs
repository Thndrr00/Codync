import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DatabaseSync } from 'node:sqlite'
const source=readFileSync(new URL('../src/store/research.rs',import.meta.url),'utf8')
function query(prefix){const match=source.match(new RegExp(`"(${prefix}[^\"]+)"`));assert.ok(match,prefix);return match[1]}
test('captured research is immutable, scoped to its bot and bounded to 200 packets',()=>{
  const db=new DatabaseSync(':memory:')
  try {
    db.exec(query('CREATE TABLE IF NOT EXISTS research_packets'))
    db.exec("CREATE TABLE bots(id TEXT PRIMARY KEY,deleted INTEGER);INSERT INTO bots VALUES('bot',0),('other-bot',0)")
    const insert=db.prepare(query('INSERT INTO research_packets'))
    const read=db.prepare(query('SELECT packet FROM research_packets'))
    const retain=db.prepare(query('DELETE FROM research_packets'))
    insert.run('other','other-bot','{"source":"other"}',1)
    for(let n=0;n<205;n++){insert.run(`packet-${n}`,'bot',JSON.stringify({source:n}),n);retain.run('bot')}
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM research_packets WHERE bot_id=?').get('bot').n,200)
    assert.equal(read.get('bot','packet-0'),undefined)
    assert.equal(read.get('other-bot','packet-204'),undefined)
    assert.equal(JSON.parse(read.get('bot','packet-204').packet).source,204)
    assert.throws(()=>insert.run('packet-204','bot','replacement',999))
    assert.ok(read.get('other-bot','other'))
    db.prepare('UPDATE bots SET deleted=1 WHERE id=?').run('bot')
    assert.equal(insert.run('after-delete','bot','{}',999).changes,0)
  } finally {db.close()}
})
