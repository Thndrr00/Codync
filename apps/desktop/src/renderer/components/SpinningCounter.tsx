import type { CSSProperties } from 'react'

const DIGITS = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

/** Idea 26: Vertical digit-reel odometer counter using tabular-nums and spring physics. */
export function SpinningCounter({
  value,
  className = '',
}: {
  value: number
  className?: string
}) {
  const text = String(Math.max(0, Math.floor(value)))
  return (
    <span className={`inline-flex items-center ${className}`.trim()} aria-label={text}>
      {Array.from(text).map((ch, idx) => {
        const digit = Number(ch)
        if (Number.isNaN(digit)) {
          return <span key={idx}>{ch}</span>
        }
        return (
          <span
            key={idx}
            aria-hidden
            className="t-reel"
            style={{ '--digit': digit } as CSSProperties}
          >
            <span className="t-reel-col">
              {DIGITS.map((d) => (
                <span key={d}>{d}</span>
              ))}
            </span>
          </span>
        )
      })}
    </span>
  )
}
