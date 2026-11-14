// How one file leaves the host: a photo WhatsApp shows inline, or a document it opens.
//
// WhatsApp crops an image in the chat unless it is told the size, and Baileys reads it only with an image
// library this bridge does not ship, so the size comes from the file's own header. The choice of message
// follows the extension, as WhatsApp does: a photo for JPEG, PNG and WebP, a document for the rest, and an
// SVG goes as an image document so WhatsApp keeps its type.
import { open } from 'node:fs/promises'
import { extname } from 'node:path'

/** How much of a file holds a PNG, WebP or JPEG size. */
const HEADER_BYTES = 256 * 1024

/** Extensions WhatsApp shows as a photo, with the type it expects. */
const IMAGE_TYPES = {
    '.jpg': 'image/jpeg',
    '.jpeg': 'image/jpeg',
    '.png': 'image/png',
    '.webp': 'image/webp',
}

/** Documents whose type is known better from the extension than from the caller. */
const DOCUMENT_TYPES = {
    '.pdf': 'application/pdf',
    '.svg': 'image/svg+xml',
}

/** The content of one `send` command: text alone, a photo with its size, or a document with its name. */
export async function sendContent(command) {
    if (!command.path) return { text: command.text ?? '' }
    const extension = (extname(command.name || '') || extname(command.path)).toLowerCase()
    const caption = command.text ? command.text : undefined
    const image = IMAGE_TYPES[extension]
    if (image) {
        // Without its size WhatsApp crops the image in the chat; a header it cannot read still sends it.
        const dimensions = await imageSize(command.path).catch(() => null)
        return { image: { url: command.path }, mimetype: image, ...dimensions, caption }
    }
    const name = String(command.name ?? `file${extension}`).slice(0, 255) || 'file'
    const mimetype = DOCUMENT_TYPES[extension] ?? command.mimetype ?? 'application/octet-stream'
    return { document: { url: command.path }, fileName: name, mimetype, caption }
}

/** The width and height of a PNG, JPEG or WebP file, read from its header without decoding it, or null. */
export async function imageSize(path) {
    const file = await open(path, 'r')
    try {
        const buffer = Buffer.alloc(HEADER_BYTES)
        const { bytesRead } = await file.read(buffer, 0, HEADER_BYTES, 0)
        const size = headerSize(buffer.subarray(0, bytesRead))
        return size && size.width > 0 && size.height > 0 ? size : null
    } finally {
        await file.close()
    }
}

function headerSize(data) {
    if (data.length >= 24 && data.readUInt32BE(0) === 0x89504e47 && data.toString('latin1', 12, 16) === 'IHDR')
        return { width: data.readUInt32BE(16), height: data.readUInt32BE(20) }
    if (data.length >= 30 && data.toString('latin1', 0, 4) === 'RIFF' && data.toString('latin1', 8, 12) === 'WEBP')
        return webpSize(data)
    if (data.length >= 4 && data[0] === 0xff && data[1] === 0xd8) return jpegSize(data)
    return null
}

function webpSize(data) {
    const chunk = data.toString('latin1', 12, 16)
    if (chunk === 'VP8 ') return { width: data.readUInt16LE(26) & 0x3fff, height: data.readUInt16LE(28) & 0x3fff }
    if (chunk === 'VP8L') {
        const bits = data.readUInt32LE(21)
        return { width: (bits & 0x3fff) + 1, height: ((bits >>> 14) & 0x3fff) + 1 }
    }
    if (chunk === 'VP8X') return { width: data.readUIntLE(24, 3) + 1, height: data.readUIntLE(27, 3) + 1 }
    return null
}

/** The size in the first frame header, after the segments before it. */
function jpegSize(data) {
    let offset = 2
    while (offset + 9 <= data.length) {
        if (data[offset] !== 0xff) return null
        const marker = data[offset + 1] ?? 0
        // Fill bytes, and the markers that have no length.
        if (marker === 0xff) offset += 1
        else if (marker === 0x01 || (marker >= 0xd0 && marker <= 0xd8)) offset += 2
        else if (marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc)
            return { width: data.readUInt16BE(offset + 7), height: data.readUInt16BE(offset + 5) }
        else offset += 2 + data.readUInt16BE(offset + 2)
    }
    return null
}
