import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DatabaseSync } from 'node:sqlite'
const schema = readFileSync(new URL('../src/store/slack.sql', import.meta.url), 'utf8')
const source = readFileSync(new URL('../src/store/slack.rs', import.meta.url), 'utf8')
function query(prefix) { const match = source.match(new RegExp(`"(${prefix}[^\"]+)"`)); assert.ok(match, prefix); return match[1] }
const enqueue = query('INSERT INTO slack_inbox')
const claim = query('UPDATE slack_inbox SET state=\'started\'')
const pending = query('SELECT event_id,generation')
const map = query('INSERT INTO slack_threads')
const outgoing = query('SELECT e.id,t.channel_id')
const claimReply = query('INSERT INTO slack_outbox')
const finishReply = query('UPDATE slack_outbox SET state=')
const status = query('SELECT entry_id,CASE')
const resume = query("UPDATE slack_inbox SET state='queued'")
const entriesSource=readFileSync(new URL('../src/store/entries.rs',import.meta.url),'utf8')
function database() {
  const db = new DatabaseSync(':memory:'); db.exec(schema)
  db.exec('CREATE TABLE entries(seq INTEGER PRIMARY KEY,id TEXT,bot_id TEXT,thread_id TEXT,kind TEXT,data TEXT)')
  return db
}
test('Slack retries and overlapping event deliveries retain one immutable incoming message', () => {
  const db = database()
  try {
    const insert = db.prepare(enqueue)
    insert.run('Ev1','generation','bot','C1','1.0','entry','original','U1',1)
    insert.run('Ev1','generation','bot','C1','1.0','entry','replacement','U1',2)
    insert.run('Ev2','generation','bot','C1','1.0','entry','overlap','U1',3)
    const rows = db.prepare(pending).all()
    assert.equal(rows.length, 1); assert.equal(rows[0].text, 'original')
  } finally { db.close() }
})
test('queued prompts can be reserved once and a changed configuration cannot reserve them', () => {
  const db = database()
  try {
    db.prepare(enqueue).run('Ev1','old','bot','C1','1.0','entry','hello','U1',1)
    assert.equal(db.prepare(claim).run('entry','new').changes, 0)
    assert.equal(db.prepare(claim).run('entry','old').changes, 1)
    assert.equal(db.prepare(claim).run('entry','old').changes, 0)
    assert.equal(db.prepare(pending).all().length, 0)
  } finally { db.close() }
})
test('only final messages in a mapped conversation become Slack replies', () => {
  const db = database()
  try {
    db.prepare(map).run('generation','C1','1.0','bot','root')
    const insert=db.prepare('INSERT INTO entries VALUES(?,?,?,?,?,?)')
    insert.run(1,'final','bot','root','agent',JSON.stringify({text:'reply',final:true}))
    insert.run(2,'stream','bot','root','agent',JSON.stringify({text:'partial',final:false}))
    insert.run(3,'private','bot',null,'agent',JSON.stringify({text:'private',final:true}))
    insert.run(4,'other-bot','other','root','agent',JSON.stringify({text:'other',final:true}))
    assert.deepEqual(db.prepare(outgoing).all('generation').map(r=>r.id),['final'])
    assert.equal(db.prepare(outgoing).all('new-generation').length,0)
  } finally { db.close() }
})
test('unknown Slack delivery outcomes cannot be automatically sent again', () => {
  const db = database()
  try {
    assert.equal(db.prepare(claimReply).run('entry',1).changes,1)
    assert.equal(db.prepare(status).all()[0].state,'unknown')
    assert.equal(db.prepare(claimReply).run('entry',2).changes,0)
    db.prepare(finishReply).run('entry','sent','123.456',3)
    db.prepare(finishReply).run('entry','unknown',null,4)
    assert.equal(db.prepare(status).all()[0].state,'sent')
    assert.equal(db.prepare(status).all()[0].external_ts,'123.456')
  } finally { db.close() }
})
test('restart preserves durable Slack messages that were not reserved for execution',()=>{
  const db=database()
  try {
    const insert=db.prepare(enqueue)
    insert.run('Ev1','generation','bot','C1','1.0','queued','one','U1',1)
    insert.run('Ev2','generation','bot','C1','2.0','reserved','two','U1',2)
    db.prepare(claim).run('reserved','generation')
    const row=db.prepare('INSERT INTO entries VALUES(?,?,?,?,?,?)')
    row.run(1,'queued','bot','root','user',JSON.stringify({source:'slack',status:'queued'}))
    row.run(2,'reserved','bot','root','user',JSON.stringify({source:'slack',status:'queued'}))
    row.run(3,'ordinary','bot',null,'user',JSON.stringify({status:'queued'}))
    const sql=entriesSource.match(/"(SELECT id FROM entries WHERE \(kind[^\"]+)"/)?.[1];assert.ok(sql)
    assert.deepEqual(db.prepare(sql).all().map(r=>r.id).sort(),['ordinary','reserved'])
  } finally{db.close()}
})
test('only a waiting sign-in message can be admitted again after owner authentication',()=>{
  const db=database()
  try {
    db.prepare(enqueue).run('Ev1','generation','bot','C1','1.0','entry','one','U1',1)
    db.prepare(claim).run('entry','generation')
    db.prepare('INSERT INTO entries VALUES(?,?,?,?,?,?)').run(1,'entry','bot','root','user',JSON.stringify({source:'slack',turnResult:{state:'waitingForSignIn'}}))
    assert.equal(db.prepare(resume).run('entry').changes,1)
    assert.equal(db.prepare(resume).run('entry').changes,0)
    db.prepare(claim).run('entry','generation')
    db.prepare("UPDATE entries SET data=json_set(data,'$.turnResult.state','done') WHERE id=?").run('entry')
    assert.equal(db.prepare(resume).run('entry').changes,0)
  }finally{db.close()}
})
