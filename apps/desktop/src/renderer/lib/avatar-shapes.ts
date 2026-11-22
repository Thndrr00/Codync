/** The avatar silhouettes a bot can pick, in the order the picker shows them. */
export const AVATAR_SHAPES: string[] = [
  'blob', 'pebble', 'squircle', 'tablet', 'wedge', 'hex', 'cloud', 'teardrop',
  'ghost', 'jellyfish', 'octopus',
]

export type AvatarShape = string

/** Whether a name from the host or a saved draft is one of the shapes this client can draw. */
export function isAvatarShape(shape: string | null | undefined): shape is AvatarShape {
  return !!shape && AVATAR_SHAPES.includes(shape)
}

/** A shape the host sent that this client doesn't know falls back to the original silhouette. */
export function safeAvatarShape(shape: string | null | undefined): AvatarShape {
  return isAvatarShape(shape) ? shape : 'blob'
}
