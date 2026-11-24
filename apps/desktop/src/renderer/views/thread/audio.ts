// Sound files sent with a message or shared by a bot play in the chat (OpenBot 0.70.0/0.72.0).
// Pure name rules: the renderer only turns the bytes into a browser player once a card is mounted.

const AUDIO = ['mp3', 'm4a', 'wav', 'aac', 'ogg', 'oga', 'opus', 'flac']

const MIME: Record<string, string> = {
  mp3: 'audio/mpeg',
  m4a: 'audio/mp4',
  wav: 'audio/wav',
  aac: 'audio/aac',
  ogg: 'audio/ogg',
  oga: 'audio/ogg',
  opus: 'audio/ogg',
  flac: 'audio/flac',
}

/** The lower-cased extension of a file name, without the dot. */
export function audioExtension(name: string): string {
  return name.split('.').pop()?.toLowerCase() ?? ''
}

/** Whether a file is a sound the chat can play in place. */
export function isAudioName(name: string): boolean {
  return AUDIO.includes(audioExtension(name))
}

/** The MIME type the browser needs to recognise the bytes behind an audio card. */
export function audioMime(name: string): string {
  return MIME[audioExtension(name)] ?? 'audio/mpeg'
}
