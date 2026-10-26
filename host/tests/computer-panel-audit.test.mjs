import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { DatabaseSync } from 'node:sqlite'
const schema = readFileSync(new URL('../src/store/schema.rs', import.meta.url), 'utf8')
const source = readFileSync(new URL('../src/store/computer_activity.rs', import.meta.url), 'utf8')
const createActivity = schema.match(/CREATE TABLE IF NOT EXISTS computer_activity\([\s\S]*?;/)?.[0]
const createActors = schema.match(/CREATE TABLE IF NOT EXISTS computer_activity_actors\([\s\S]*?;/)?.[0]
const cleanup = schema.match(/CREATE TRIGGER IF NOT EXISTS computer_activity_cleanup_actor[\s\S]*?END;/)?.[0]
const record = source.match(/"(INSERT INTO computer_activity\(bot_id[^\"]+)"/)?.[1]
const read = source.match(/"(SELECT a.id, a.bot_id[^\"]+)"/)?.[1]
const retain = source.match(/"(DELETE FROM computer_activity WHERE bot_id[^\"]+)"/)?.[1]
for (const query of [createActivity, createActors, cleanup, record, read, retain]) assert.ok(query)
function database() {
  const db = new DatabaseSync(':memory:')
  db.exec('CREATE TABLE bots(id TEXT PRIMARY KEY,deleted INTEGER DEFAULT 0);')
  db.exec(createActivity); db.exec(createActors)
  db.exec(cleanup)
  db.prepare('INSERT INTO bots(id) VALUES(?)').run('bot')
  return db
}
test('legacy activity defaults to bot identity while new owner activity retains its real actor', () => {
  const db = database()
  try {
    db.prepare(record).run('bot', 'click', 'success', 1)
    const saved = db.prepare(record).run('bot', 'workspace_shell', 'success', 2)
    db.prepare('INSERT INTO computer_activity_actors VALUES(?,?,?)').run(saved.lastInsertRowid, 'device', 'paired-device-key')
    const rows = db.prepare(read).all('bot', 1000)
    assert.equal(Object.values(rows[0]).at(-2), 'device')
    assert.equal(Object.values(rows[0]).at(-1), 'paired-device-key')
    assert.equal(Object.values(rows[1]).at(-2), 'bot')
    assert.equal(Object.values(rows[1]).at(-1), 'bot')
  } finally { db.close() }
})
test('activity retention also removes associated actor rows', () => {
  const db = database()
  try {
    for (let i = 0; i < 4; i++) {
      const saved = db.prepare(record).run('bot', 'workspace_read', 'success', i)
      db.prepare('INSERT INTO computer_activity_actors VALUES(?,?,?)').run(saved.lastInsertRowid, 'owner', 'local')
    }
    db.prepare(retain).run('bot', 2)
    assert.equal(db.prepare('SELECT COUNT(*) AS count FROM computer_activity_actors').get().count, 2)
  } finally { db.close() }
})
test('deleted bots cannot regain activity after an in-flight action completes', () => {
  const db = database()
  try {
    db.prepare('UPDATE bots SET deleted=1').run()
    assert.equal(db.prepare(record).run('bot', 'workspace_shell', 'failed', 5).changes, 0)
    assert.equal(db.prepare('SELECT COUNT(*) AS count FROM computer_activity').get().count, 0)
  } finally { db.close() }
})
