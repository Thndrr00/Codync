import type { ReactNode } from 'react'

export function IslandPanel({ children }: { children: ReactNode }) {
  return <div className="island-card island-control-panel" tabIndex={0}>{children}</div>
}
