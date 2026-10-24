import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DatabaseSync } from 'node:sqlite'
const source=readFileSync(new URL('../src/store/setup.rs',import.meta.url),'utf8')
test('setup milestones are claimed once and remain separate from unrelated settings',()=>{
  const sql=source.match(/"(INSERT INTO kv[^\"]+)"/)?.[1];assert.ok(sql)
  const db=new DatabaseSync(':memory:')
  try {
    db.exec("CREATE TABLE kv(k TEXT PRIMARY KEY,v TEXT);INSERT INTO kv VALUES('analytics_enabled','0')")
    const claim=db.prepare(sql)
    assert.equal(claim.run('ready').changes,1)
    assert.equal(claim.run('ready').changes,0)
    assert.equal(claim.run('activated').changes,1)
    assert.equal(db.prepare("SELECT v FROM kv WHERE k='analytics_enabled'").get().v,'0')
  } finally {db.close()}
})
