/** What file a form field of each type takes, and the button that picks it. Pure helpers. */
import { isAudioName, isVideoName } from '../../thread/media.ts'

export type MediaInput = 'image' | 'video' | 'audio' | 'file'

/** A picture a `LoadImage` reads: the formats the app shows in the chat too. */
const PICTURES = ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif']

/** Any other file a node reads by name: a splat scene, a camera pack, a mesh. */
const FILES = ['ply', 'npz', 'spz', 'splat', 'obj', 'glb', 'gltf', 'zip']

/** Whether a template field of this type accepts `name`. */
export function accepts(type: MediaInput, name: string): boolean {
  if (type === 'video') return isVideoName(name) || name.toLowerCase().endsWith('.gif')
  if (type === 'audio') return isAudioName(name)
  if (type === 'file') return FILES.includes(name.split('.').pop()?.toLowerCase() ?? '')
  return PICTURES.includes(name.split('.').pop()?.toLowerCase() ?? '')
}

/** The word the picker uses, in the singular the field needs ("a video", "two pictures"). */
export function fieldWord(type: MediaInput): string {
  return type === 'video' ? 'video' : type === 'audio' ? 'sound' : type === 'file' ? 'scene file' : 'picture'
}

/** The button's text: choosing the same thing again says so. */
export function pickLabel(type: MediaInput, picked: boolean): string {
  const word = fieldWord(type)
  return picked ? `Choose another ${word}` : `Choose a ${word}`
}
