import { useCallback, useState } from 'react'
import { createPortal } from 'react-dom'
import { Icon } from './Icon'

export interface ToastItem {
  id: string
  text: string
  tone?: 'info' | 'success' | 'danger'
  leaving?: boolean
}

/** Idea 29: Stackable toast notifications with spring entry and exit animations. */
export function useToasts() {
  const [toasts, setToasts] = useState<ToastItem[]>([])

  const dismiss = useCallback((id: string) => {
    setToasts((prev) => prev.map((t) => (t.id === id ? { ...t, leaving: true } : t)))
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id))
    }, 180)
  }, [])

  const push = useCallback(
    (text: string, tone: ToastItem['tone'] = 'info', durationMs = 3200) => {
      const id = `toast_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`
      setToasts((prev) => [...prev.slice(-4), { id, text, tone }])
      if (durationMs > 0) {
        setTimeout(() => dismiss(id), durationMs)
      }
      return id
    },
    [dismiss],
  )

  return { toasts, push, dismiss }
}

export function ToastContainer({
  toasts,
  onDismiss,
}: {
  toasts: ToastItem[]
  onDismiss: (id: string) => void
}) {
  if (toasts.length === 0) return null
  return createPortal(
    <div className="t-toast-stack" role="region" aria-label="Notifications">
      {toasts.map((t) => (
        <div
          key={t.id}
          role="status"
          className={`t-toast ${t.leaving ? 'is-leaving' : ''}`}
        >
          <Icon
            name={
              t.tone === 'danger'
                ? 'exclamationmark.triangle.fill'
                : t.tone === 'success'
                  ? 'checkmark.circle.fill'
                  : 'info.circle.fill'
            }
            size={14}
            color={
              t.tone === 'danger'
                ? 'var(--danger)'
                : t.tone === 'success'
                  ? 'var(--added)'
                  : 'var(--secondary)'
            }
          />
          <span className="spacer">{t.text}</span>
          <button
            type="button"
            aria-label="Dismiss"
            onClick={() => onDismiss(t.id)}
            style={{ opacity: 0.65 }}
          >
            <Icon name="xmark" size={11} />
          </button>
        </div>
      ))}
    </div>,
    document.body,
  )
}
