import { ProgressBar } from '../../components/Controls'
import { usagePercent } from '../../lib/usage-dots'
import './usage.css'

/** A dotted meter: the provider's tint below 70%, then amber, red from 90%. `pace` draws the even-spend
 * mark, the share the limit would be at now if it were spent evenly until its reset (OpenBot 0.72.0). */
export function UsageBar({ percent, tint, pace }: { percent: number; tint: string; pace?: number | null }) {
  const value = usagePercent(percent)
  const fill = value >= 90 ? 'var(--danger)' : value >= 70 ? 'var(--warning)' : tint
  return (
    <div className="usage-bar-wrap">
      <ProgressBar
        value={value / 100}
        label={`${Math.round(value)}% used`}
        fill={fill}
        className="usage-bar"
      />
      {typeof pace === 'number' ? <span className="usage-pace" data-ahead={value > pace} style={{ left: `${Math.min(100, pace)}%` }} aria-hidden /> : null}
    </div>
  )
}
