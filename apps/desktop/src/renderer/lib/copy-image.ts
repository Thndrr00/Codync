/**
 * Copies a picture to the clipboard (OpenBot 0.67.0: "You can copy an image from a chat to the
 * clipboard"). Chromium writes PNG, so a JPEG, WebP or SVG is drawn once and encoded as PNG.
 */

/** What the blob is read as: SVG needs its own type or the browser shows nothing. */
export function pictureType(name: string): string {
  return name.toLowerCase().endsWith('.svg') ? 'image/svg+xml' : ''
}

async function asPng(blob: Blob): Promise<Blob> {
  if (blob.type === 'image/png') return blob
  const bitmap = await createImageBitmap(blob)
  try {
    const canvas = document.createElement('canvas')
    canvas.width = bitmap.width
    canvas.height = bitmap.height
    const context = canvas.getContext('2d')
    if (!context) throw new Error('The picture could not be drawn')
    context.drawImage(bitmap, 0, 0)
    return await new Promise<Blob>((resolve, reject) =>
      canvas.toBlob((png) => (png ? resolve(png) : reject(new Error('The picture could not be copied'))), 'image/png'),
    )
  } finally {
    bitmap.close()
  }
}

/** Puts a picture on the clipboard, converting it to PNG first when it is not one. */
export async function copyPicture(data: Uint8Array): Promise<void> {
  const blob = new Blob([data as BlobPart])
  const png = await asPng(blob)
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': png })])
}
