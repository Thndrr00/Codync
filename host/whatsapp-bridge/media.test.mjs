// One `send` command of the bridge: what leaves for WhatsApp, by the file's extension.
import assert from 'node:assert/strict'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test, { after } from 'node:test'
import { sendContent } from './media.mjs'

const folder = mkdtempSync(join(tmpdir(), 'codync-whatsapp-media-'))

/** A JPEG header whose first frame says 32 x 64, without the image data. */
function jpeg() {
    return Buffer.from([
        0xff, 0xd8, // start of image
        0xff, 0xc0, 0x00, 0x11, 0x08, // frame header, 17 bytes, 8 bits per sample
        0x00, 0x40, // height, 64
        0x00, 0x20, // width, 32
        0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01,
    ])
}

/** A PNG header saying 120 x 80. */
function png() {
    const data = Buffer.alloc(26)
    data.writeUInt32BE(0x89504e47, 0)
    data.writeUInt32BE(0x0d0a1a0a, 4)
    data.writeUInt32BE(13, 8)
    data.write('IHDR', 12, 'latin1')
    data.writeUInt32BE(120, 16)
    data.writeUInt32BE(80, 20)
    return data
}

const file = (name, bytes) => {
    const path = join(folder, name)
    writeFileSync(path, bytes)
    return path
}

test('a JPEG goes as a photo with its type and its size', async () => {
    const path = file('photo.jpg', jpeg())
    const content = await sendContent({ path, name: 'photo.jpg', text: 'Look' })
    assert.equal(content.image.url, path)
    assert.equal(content.mimetype, 'image/jpeg')
    assert.equal(content.width, 32)
    assert.equal(content.height, 64)
    assert.equal(content.caption, 'Look')
    assert.equal(content.document, undefined)
})

test('a PNG goes as a photo with the size in its header', async () => {
    const path = file('shot.png', png())
    const content = await sendContent({ path, name: 'shot.png', text: '' })
    assert.equal(content.mimetype, 'image/png')
    assert.equal(content.width, 120)
    assert.equal(content.height, 80)
    assert.equal(content.caption, undefined, 'no empty caption')
})

test('an SVG goes as a document WhatsApp keeps as an image', async () => {
    const path = file('logo.svg', '<svg xmlns="http://www.w3.org/2000/svg"/>')
    const content = await sendContent({ path, name: 'logo.svg', mimetype: 'application/octet-stream' })
    assert.equal(content.document.url, path)
    assert.equal(content.fileName, 'logo.svg')
    assert.equal(content.mimetype, 'image/svg+xml')
    assert.equal(content.image, undefined)
})

test('a PDF keeps its own type', async () => {
    const path = file('report.pdf', '%PDF-1.4')
    const content = await sendContent({ path, name: 'report.pdf', mimetype: 'application/pdf' })
    assert.equal(content.document.url, path)
    assert.equal(content.fileName, 'report.pdf')
    assert.equal(content.mimetype, 'application/pdf')
})

test('an unknown extension keeps the type the caller gave, and the name it chose', async () => {
    const path = file('data.bin', 'binary')
    const content = await sendContent({ path, name: 'data.bin', mimetype: 'text/plain' })
    assert.equal(content.mimetype, 'text/plain')
    assert.equal(content.fileName, 'data.bin')
})

test('text alone sends no file', async () => {
    assert.deepEqual(await sendContent({ text: 'hello' }), { text: 'hello' })
})

after(() => rmSync(folder, { recursive: true, force: true }))
