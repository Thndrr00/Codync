import type { ReactNode } from 'react'

/** Idea 16: Shimmer text sweep while a bot is actively thinking or running a tool. */
export function ShimmerText({
  children,
  active = true,
  className = '',
}: {
  children: ReactNode
  active?: boolean
  className?: string
}) {
  return (
    <span className={`t-shimmer ${active ? 'is-active' : ''} ${className}`.trim()}>
      {children}
    </span>
  )
}
