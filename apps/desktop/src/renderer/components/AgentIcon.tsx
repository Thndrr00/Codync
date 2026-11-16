import { Icon } from './Icon'
import './agent-icon.css'

// Agents' logos by ACP registry id, or by Codync's id for agents outside the registry
// (template images: drawn in the current color).
const icons = import.meta.glob<string>('../assets/agents/*.svg', { eager: true, query: '?url', import: 'default' })
const byId = new Map(Object.entries(icons).map(([path, url]) => [path.split('/').pop()!.replace('.svg', ''), url]))

export function agentIconURL(registry: string | null | undefined) {
  return registry ? (byId.get(registry) ?? null) : null
}

/** Registry logos are one-colour icons: grey at rest, an accent while chosen, never a brand gradient. */
export function agentTint(_registry: string | null | undefined) {
  return 'var(--secondary)'
}

/** An agent's logo from the ACP registry, or a generic terminal when it has none. One colour at rest. */
export function AgentIcon({ registry, size, color = 'var(--secondary)' }: { registry?: string | null; size: number; color?: string }) {
  const url = agentIconURL(registry)
  if (!url) return <Icon name="terminal" size={size * 0.8} scaled={false} color={color} />
  return <span aria-hidden className="agent-icon" style={{ width: size, height: size, background: color, WebkitMaskImage: `url("${url}")` }} />
}
