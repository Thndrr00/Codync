import { useEffect, useState } from 'react'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import { SwitchRow } from '../bots/parts'

/** The opt-in that lets successful browser teaching store one site fact per page (OpenBot #14). */
export function BrowsingLearningRow() {
  const tr = useTr()
  const local = useApp().local
  const [enabled, setEnabled] = useState(false)

  useEffect(() => {
    if (!local) return
    let live = true
    void local.ready()
      .then((client) => client.call<{ enabled: boolean }>('browsingLearning'))
      .then((answer) => { if (live) setEnabled(answer.enabled === true) })
      .catch(() => { /* keep the safe default: off */ })
    return () => { live = false }
  }, [local])

  const change = (on: boolean) => {
    setEnabled(on)
    if (!local) return
    void local.ready().then((client) => client.call('setBrowsingLearning', { enabled: on })).catch(() => setEnabled(!on))
  }

  return (
    <SwitchRow
      title={tr('Learn from my browsing')}
      detail={tr('Off by default. When a browser task succeeds, the agent can remember how that site works and use it on later visits. Only changes this device.')}
      on={enabled}
      onChange={change}
    />
  )
}
