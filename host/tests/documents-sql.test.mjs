// Validate production persistence queries without building the Rust host.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, mkdtempSync, unlinkSync, rmdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { DatabaseSync } from 'node:sqlite'

const schemaSource = readFileSync(new URL('../src/store/documents/schema.rs', import.meta.url), 'utf8')
const schema = schemaSource.match(/db\.execute_batch\(\s*"([\s\S]*?)",\s*\)/)?.[1]
assert.ok(schema, 'production document schema must exist')
const storeSource = readFileSync(new URL('../src/store/documents/mod.rs', import.meta.url), 'utf8')
const reviewsSource = readFileSync(new URL('../src/store/documents/reviews.rs', import.meta.url), 'utf8')
const retryQuery = storeSource.match(/"(SELECT document FROM document_creations[^\"]+)"/)?.[1]
const pendingCount = reviewsSource.match(/"(SELECT COUNT\(\*\) FROM document_reviews WHERE bot_id = \?1 AND status = 'pending'[^\"]+)"/)?.[1]
const treeQuery = storeSource.match(/"(WITH RECURSIVE tree\(id\)[^\"]+)"/)?.[1]
assert.ok(retryQuery && pendingCount && treeQuery, 'production document queries must exist')

test('creation receipts preserve their original payload across reopening and isolate libraries', () => {
  const directory = mkdtempSync(join(tmpdir(), 'codync-documents-'))
  const path = join(directory, 'documents.sqlite')
  let db = new DatabaseSync(path)
  try {
    db.exec(schema)
    db.prepare('INSERT INTO document_creations VALUES(?,?,?)').run('a', 'nonce', JSON.stringify({ id: 'original', body: '**Exact**\r\n日本語' }))
    db.prepare('INSERT INTO document_creations VALUES(?,?,?)').run('b', 'nonce', JSON.stringify({ id: 'other' }))
    db.close(); db = new DatabaseSync(path)
    assert.equal(JSON.parse(db.prepare(retryQuery).get('a', 'nonce').document).body, '**Exact**\r\n日本語')
    assert.equal(JSON.parse(db.prepare(retryQuery).get('b', 'nonce').document).id, 'other')
    assert.throws(() => db.prepare('INSERT INTO document_creations VALUES(?,?,?)').run('a', 'nonce', '{}'))
  } finally { db.close(); unlinkSync(path); rmdirSync(directory) }
})

test('review capacity counts only unexpired pending proposals rather than completed history', () => {
  const db = new DatabaseSync(':memory:')
  try {
    db.exec(schema)
    const insert = db.prepare('INSERT INTO document_reviews VALUES(?,?,?,?,?,?,?,?,?)')
    for (let index = 0; index < 1001; index++) insert.run(`done-${index}`, 'a', `effect-${index}`, 'hash', '{}', 'declined', null, 1, 1000)
    insert.run('pending', 'a', 'pending-effect', 'hash', '{}', 'pending', null, 1, 1000)
    insert.run('expired', 'a', 'expired-effect', 'hash', '{}', 'pending', null, 1, 2)
    assert.equal(Object.values(db.prepare(pendingCount).get('a', 100))[0], 1)
  } finally { db.close() }
})

test('recursive deletion query includes descendants from the owned library only', () => {
  const db = new DatabaseSync(':memory:')
  try {
    db.exec(schema)
    const insert = db.prepare('INSERT INTO documents(id,bot_id,parent_id,title,body,revision,rev,created_at,updated_at) VALUES(?,?,?,?,?,1,1,1,1)')
    insert.run('parent', 'a', null, 'Parent', 'Body')
    insert.run('child', 'a', 'parent', 'Child', 'Body')
    insert.run('grandchild', 'a', 'child', 'Grandchild', 'Body')
    insert.run('foreign', 'b', 'parent', 'Foreign', 'Body')
    assert.deepEqual(db.prepare(treeQuery).all('parent', 'a').map((row) => row.id), ['parent', 'child', 'grandchild'])
  } finally { db.close() }
})
