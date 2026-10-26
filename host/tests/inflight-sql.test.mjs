import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DatabaseSync } from 'node:sqlite'
const source=readFileSync(new URL('../src/store/inflight.rs',import.meta.url),'utf8')
const put=source.match(/"(INSERT INTO kv[^\"]+)"/)?.[1]
const remove=source.match(/"(DELETE FROM kv[^\"]+)"/)?.[1]
test('the turn marker and source identities commit together and a failed source write preserves old state',()=>{
  assert.ok(put);assert.ok(remove)
  const db=new DatabaseSync(':memory:')
  try {
    db.exec('CREATE TABLE kv(k TEXT PRIMARY KEY,v TEXT)')
    db.prepare(put).run('turn.inflight.bot','old')
    db.exec("CREATE TRIGGER reject_sources BEFORE INSERT ON kv WHEN new.k='turn.sources.bot' BEGIN SELECT RAISE(ABORT,'simulated storage failure'); END")
    db.exec('BEGIN')
    db.prepare(put).run('turn.inflight.bot','new')
    assert.throws(()=>db.prepare(put).run('turn.sources.bot','{"entries":["request"]}'))
    db.exec('ROLLBACK')
    assert.equal(db.prepare('SELECT v FROM kv WHERE k=?').get('turn.inflight.bot').v,'old')
    db.exec('DROP TRIGGER reject_sources;BEGIN')
    db.prepare(put).run('turn.inflight.bot','new');db.prepare(put).run('turn.sources.bot','{"entries":["request"]}');db.exec('COMMIT')
    assert.equal(JSON.parse(db.prepare('SELECT v FROM kv WHERE k=?').get('turn.sources.bot').v).entries[0],'request')
    db.prepare(remove).run('turn.sources.bot')
    assert.equal(db.prepare('SELECT v FROM kv WHERE k=?').get('turn.sources.bot'),undefined)
  }finally{db.close()}
})
