export const ONBOARDING_STEPS = ['welcome', 'account', 'computer_access', 'pairing', 'ready'] as const
export type OnboardingStep = typeof ONBOARDING_STEPS[number]
type Event = { event: 'onboarding_step_viewed' | 'onboarding_abandoned'; step: OnboardingStep; clientEventId: string }
type State = { active: OnboardingStep | null; pending: Event[] }
const KEY = 'windowsOnboardingTelemetry'

/** Persist before sending; late acknowledgements remove only their own immutable event IDs. */
export class OnboardingTracker {
  private sending = false
  private storage: Pick<Storage, 'getItem' | 'setItem'>
  private send: (event: Event) => Promise<unknown>
  private enabled: () => boolean
  constructor(storage: Pick<Storage, 'getItem' | 'setItem'>, send: (event: Event) => Promise<unknown>, enabled: () => boolean = () => true) {
    this.storage = storage; this.send = send; this.enabled = enabled
  }
  private read(): State {
    try {
      const state = JSON.parse(this.storage.getItem(KEY) ?? '{}')
      const validStep = (step: unknown): step is OnboardingStep => ONBOARDING_STEPS.includes(step as OnboardingStep)
      return {
        active: validStep(state.active) ? state.active : null,
        pending: Array.isArray(state.pending) ? state.pending.filter((item: Event) => item && validStep(item.step) &&
          ['onboarding_step_viewed', 'onboarding_abandoned'].includes(item.event) && typeof item.clientEventId === 'string' &&
          /^[0-9a-f-]{36}$/i.test(item.clientEventId)).slice(-64) : [],
      }
    } catch { return { active: null, pending: [] } }
  }
  private write(state: State) { this.storage.setItem(KEY, JSON.stringify(state)) }
  private append(state: State, event: Event['event'], step: OnboardingStep) {
    state.pending.push({ event, step, clientEventId: crypto.randomUUID() })
    state.pending = state.pending.slice(-64)
  }
  recover() { this.abandon() }
  step(step: OnboardingStep | null) {
    if (!this.enabled()) { this.clear(); return }
    const state = this.read()
    if (state.active === step) return
    state.active = step
    if (step !== null) this.append(state, 'onboarding_step_viewed', step)
    this.write(state)
    void this.flush()
  }
  abandon() {
    if (!this.enabled()) { this.clear(); return }
    const state = this.read()
    if (state.active === null || state.active === 'ready') return
    this.append(state, 'onboarding_abandoned', state.active)
    state.active = null
    this.write(state)
    void this.flush()
  }
  clear() { this.write({ active: null, pending: [] }) }
  async flush() {
    if (!this.enabled()) { this.clear(); return }
    if (this.sending) return
    this.sending = true
    try {
      for (const event of this.read().pending) {
        if (!this.enabled()) { this.clear(); break }
        await this.send(event)
        const latest = this.read()
        latest.pending = latest.pending.filter((item) => item.clientEventId !== event.clientEventId)
        this.write(latest)
      }
    } catch { /* The durable event remains for a later connection; never invent a successful delivery. */ }
    finally { this.sending = false }
  }
}
