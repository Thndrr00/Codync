import { useEffect, useRef } from 'react'
import { OnboardingTracker, type OnboardingStep } from '../lib/onboarding-tracker'
import { useApp } from '../store/context'

export function useOnboardingTelemetry(step: OnboardingStep | null) {
  const app = useApp()
  const tracker = useRef<OnboardingTracker | null>(null)
  const current = useRef(step)
  current.current = step
  useEffect(() => {
    if (window.codync.platform !== 'win32') return
    const model = new OnboardingTracker(localStorage, async (event) => {
      if (!app.local) throw new Error('The local host is unavailable')
      const client = await app.local.ready()
      if (app.local.analytics === false) return
      return client.call('track', event)
    }, () => app.local?.analytics !== false)
    tracker.current = model
    model.recover()
    model.step(current.current)
    const interval = setInterval(() => void model.flush(), 30000)
    const hidden = () => { if (document.hidden) model.abandon(); else model.step(current.current) }
    document.addEventListener('visibilitychange', hidden)
    return () => { clearInterval(interval); document.removeEventListener('visibilitychange', hidden); model.abandon(); tracker.current = null }
  }, [app])
  useEffect(() => { tracker.current?.step(step) }, [step])
}
