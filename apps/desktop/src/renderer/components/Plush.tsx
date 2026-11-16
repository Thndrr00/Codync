import { CharacterAvatar } from './Avatar'

/**
 * The felt plush of an empty list or search: it blinks and looks at the pointer, and stays a
 * still image with Reduce Motion (CharacterAvatar stops its loop). Decoration: the text under
 * it says what is empty.
 */
export function Plush({ size = 64 }: { size?: number }) {
  return (
    <div className="plush" aria-hidden>
      <CharacterAvatar shape="blob" color="violet" size={size} mood="working" />
    </div>
  )
}
