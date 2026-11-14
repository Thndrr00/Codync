import { useEffect, useState } from 'react'
import { Dialog } from '../components/Overlay'
import { useModel } from '../lib/observable'
import { useTr } from '../lib/use-language'
import { useApp } from '../store/context'

/**
 * "Ask on this computer": a paired device wants to control this computer. The person here allows or
 * denies it within 30 seconds; the phone waits for the answer and gives up when the window closes.
 * Only this computer's own app shows this, and only this computer can answer it.
 */
export function RemoteControlPrompt() {
  const app = useApp()
  const local = useModel(app.local)
  const tr = useTr()
  const consent = local?.screen?.consent ?? null
  const [, tick] = useState(0)
  useEffect(() => {
    if (!consent) return
    const id = window.setInterval(() => tick((n) => n + 1), 250)
    return () => window.clearInterval(id)
  }, [consent])
  const seconds = consent ? Math.max(0, Math.ceil((consent.expiresAt - Date.now()) / 1000)) : 0
  const answer = (allow: boolean) => {
    if (consent) void local?.answerScreenRequest(consent.id, allow)
  }
  return (
    <Dialog
      open={!!consent && seconds > 0}
      title={tr('Allow remote control?')}
      message={tr(
        'A device you paired wants to control this computer. It starts only if you allow it within {seconds} seconds.',
      ).replace('{seconds}', String(seconds))}
      actions={[{ title: 'Allow', action: () => answer(true) }]}
      cancel={tr('Deny')}
      onCancel={() => answer(false)}
      onClose={() => {}}
    />
  )
}
