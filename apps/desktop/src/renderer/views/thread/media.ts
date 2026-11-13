/** The file kinds the chat plays or draws, by extension. Pure helpers, so they are tested without JSX. */

/** The symbol for a file by its extension. */
export function attachmentSymbol(name: string) {
  const ext = name.split('.').pop()?.toLowerCase() ?? ''
  if (['png', 'jpg', 'jpeg', 'heic', 'gif', 'webp', 'tiff', 'bmp', 'svg'].includes(ext)) return 'photo'
  if (ext === 'pdf') return 'doc.richtext'
  if (['zip', 'gz', 'tar', '7z'].includes(ext)) return 'doc.zipper'
  if (['mov', 'mp4', 'm4v', 'webm', 'mkv', 'avi'].includes(ext)) return 'film'
  if (['mp3', 'm4a', 'wav', 'aac', 'flac', 'ogg', 'opus'].includes(ext)) return 'waveform'
  if (['glb', 'gltf', 'obj', 'ply', 'stl', 'splat', 'ksplat', 'spz'].includes(ext)) return 'shippingbox'
  return 'doc'
}

const videoExtensions = ['mp4', 'webm', 'mov', 'm4v', 'mkv', 'avi']
const audioExtensions = ['mp3', 'm4a', 'wav', 'aac', 'flac', 'ogg', 'opus']
const meshExtensions = ['glb', 'gltf', 'obj', 'ply', 'stl', 'splat', 'ksplat', 'spz']

const extension = (name: string) => name.split('.').pop()?.toLowerCase() ?? ''

/** A video ComfyUI produced: played in the chat instead of only offered as a download. */
export const isVideoName = (name: string) => videoExtensions.includes(extension(name))

/** An audio ComfyUI produced: played in the chat. */
export const isAudioName = (name: string) => audioExtensions.includes(extension(name))

/** A mesh or splat: `.glb` is drawn, the rest keep their type and a download. */
export const isMeshName = (name: string) => meshExtensions.includes(extension(name))

/** The MIME a blob needs for `<video>`/`<audio>` to pick the right decoder. */
export const mediaMime = (name: string): string => {
  const video: Record<string, string> = { mp4: 'video/mp4', webm: 'video/webm', mov: 'video/quicktime', m4v: 'video/mp4', mkv: 'video/x-matroska', avi: 'video/x-msvideo' }
  const audio: Record<string, string> = { mp3: 'audio/mpeg', m4a: 'audio/mp4', wav: 'audio/wav', aac: 'audio/aac', flac: 'audio/flac', ogg: 'audio/ogg', opus: 'audio/ogg' }
  return video[extension(name)] ?? audio[extension(name)] ?? ''
}

export function byteCount(bytes: number) {
  if (bytes < 1000) return `${bytes} bytes`
  const units = ['KB', 'MB', 'GB']
  let value = bytes / 1000
  let unit = 0
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000
    unit++
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`
}
