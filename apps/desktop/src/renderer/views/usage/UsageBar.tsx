import { ProgressBar } from '../../components/Controls'
import { usagePercent } from '../../lib/usage-dots'
import './usage.css'

/** A dotted meter: the provider's tint below 70%, then amber, red from 90%. */
export function UsageBar({ percent, tint }: { percent: number; tint: string }) {
  const value = usagePercent(percent)
  const fill = value >= 90 ? 'var(--danger)' : value >= 70 ? 'var(--warning)' : tint
  return (
    <ProgressBar
      value={value / 100}
      label={`${Math.round(value)}% used`}
      fill={fill}
      className="usage-bar"
    />
  )
}
