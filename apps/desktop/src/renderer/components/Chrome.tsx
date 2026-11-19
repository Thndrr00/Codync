import { type RefObject } from 'react'
import { useTr } from '../lib/use-language'
import { Icon } from './Icon'
import './chrome.css'

/**
 * The one chrome icon button for the window's top-row toggles. The sidebar and side-panel toggles
 * use it so they have one icon size, one shape and one hover fill. It is plain at rest and only
 * fills under the pointer (Motion through the theme's `--hover`/`--morph` tokens); there is no
 * tooltip — the accessible name is the only label, as on the other chrome buttons.
 */
export function ChromeIconButton({ icon, label, onClick, size = 32, selected = false, ringed = false, disabled, className, buttonRef, menuOpen }: {
  icon: string
  label: string
  onClick?: () => void
  size?: number
  selected?: boolean
  /** The soft page-color ring of the chat header buttons (the side-panel toggle's look). */
  ringed?: boolean
  disabled?: boolean
  className?: string
  buttonRef?: RefObject<HTMLButtonElement | null>
  menuOpen?: boolean
}) {
  const tr = useTr()
  return (
    <button
      ref={buttonRef}
      type="button"
      className={`chrome-icon-button${ringed ? ' ringed' : ''}${selected ? ' selected' : ''}${className ? ` ${className}` : ''}`}
      style={{ width: size, height: size }}
      aria-label={tr(label)}
      aria-haspopup={menuOpen === undefined ? undefined : 'menu'}
      aria-expanded={menuOpen}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name={icon} size={size / 2} weight="medium" />
    </button>
  )
}

/**
 * A settings page's Close: a plain X with no grey pad and no tooltip (only an accessible name),
 * filling when the pointer moves over it, like the other top-row chrome controls.
 */
export function ModalCloseButton({ onClick, label = 'Close' }: { onClick?: () => void; label?: string }) {
  const tr = useTr()
  return (
    <button type="button" className="modal-close-button" aria-label={tr(label)} onClick={onClick}>
      <Icon name="xmark" size={14} weight="medium" />
    </button>
  )
}
