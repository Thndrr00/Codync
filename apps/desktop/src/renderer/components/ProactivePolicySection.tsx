import {
  ACTION_CLASSES,
  BOT_AVATAR_LOOKS,
  WORKER_ROLE_TEMPLATES,
  defaultProactiveSettings,
  type ActionClass,
  type BotAvatarLook,
  type ProactiveSettings,
  type RulePolicy,
  type WorkerRoleTemplate,
} from '@shared/models'
import { avatarColor } from '../lib/theme'
import { Switch } from './Controls'
import { RobotAvatar } from './RobotAvatar'
import { SlidingTabs } from './SlidingTabs'

const POLICY_TABS: Array<{ id: RulePolicy; label: string }> = [
  { id: 'allow', label: 'Permitir' },
  { id: 'ask', label: 'Preguntar' },
  { id: 'block', label: 'Bloquear' },
]

export interface ProactivePolicyProps {
  avatarColorId: string
  avatarLook?: string
  actionRules?: Partial<Record<ActionClass, RulePolicy>>
  proactive?: ProactiveSettings
  onChangeLook: (look: string | undefined) => void
  onChangeRule: (cls: ActionClass, policy: RulePolicy) => void
  onChangeProactive: (next: ProactiveSettings) => void
  onApplyRoleTemplate?: (template: WorkerRoleTemplate) => void
}

/**
 * Configures the 12 geometric robot avatar looks, 7 semantic action-class guardrails
 * (allow / ask / block), and proactive routine settings (quiet hours, daily cap, background apps).
 */
export function ProactivePolicySection({
  avatarColorId,
  avatarLook,
  actionRules = {},
  proactive = defaultProactiveSettings(),
  onChangeLook,
  onChangeRule,
  onChangeProactive,
  onApplyRoleTemplate,
}: ProactivePolicyProps) {
  const colorHex = avatarColor(avatarColorId)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      {onApplyRoleTemplate ? (
        <div>
          <div className="eyebrow" style={{ marginBottom: 8 }}>Perfiles de Rol Especializado (N15)</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            {WORKER_ROLE_TEMPLATES.map((tpl) => (
              <button
                key={tpl.id}
                type="button"
                className="btn ghost sm"
                title={tpl.summary}
                onClick={() => onApplyRoleTemplate(tpl)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 6,
                  padding: '5px 10px',
                  borderRadius: 10,
                  background: 'var(--bg-subtle, rgba(127, 127, 127, 0.08))',
                }}
              >
                <RobotAvatar look={tpl.avatarLook} color={avatarColor(tpl.avatarColor)} size={18} />
                <span style={{ fontSize: 12, fontWeight: 500 }}>{tpl.label}</span>
              </button>
            ))}
          </div>
        </div>
      ) : null}

      <div>
        <div className="eyebrow" style={{ marginBottom: 8 }}>Apariencia Robot (12 looks geométricos)</div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
          {BOT_AVATAR_LOOKS.map((look: BotAvatarLook) => {
            const active = avatarLook === look.id
            return (
              <button
                key={look.id}
                type="button"
                className="btn ghost sm"
                onClick={() => onChangeLook(active ? undefined : look.id)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 6,
                  padding: '4px 8px',
                  borderRadius: 10,
                  background: active ? 'var(--bg-selected, rgba(127, 127, 127, 0.16))' : undefined,
                }}
              >
                <RobotAvatar look={look.id} color={colorHex} size={22} />
                <span style={{ fontSize: 12 }}>{look.label}</span>
              </button>
            )
          })}
        </div>
      </div>

      <div>
        <div className="eyebrow" style={{ marginBottom: 8 }}>Permisos por clase de acción</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {ACTION_CLASSES.map((item) => {
            const current: RulePolicy = actionRules[item.id] ?? (item.id === 'browse' || item.id === 'apps' ? 'allow' : 'ask')
            return (
              <div
                key={item.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 12,
                  padding: '6px 10px',
                  borderRadius: 10,
                  background: 'var(--bg-subtle, rgba(127, 127, 127, 0.06))',
                }}
              >
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, fontWeight: 500 }}>{item.label}</div>
                  <div className="muted" style={{ fontSize: 11 }}>{item.hint}</div>
                </div>
                <SlidingTabs
                  items={POLICY_TABS}
                  value={current}
                  onChange={(policy) => onChangeRule(item.id, policy)}
                />
              </div>
            )
          })}
        </div>
      </div>

      <div>
        <div className="eyebrow" style={{ marginBottom: 8 }}>Rutina Proactiva y Horas de Silencio</div>
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 10,
            padding: 12,
            borderRadius: 12,
            background: 'var(--bg-subtle, rgba(127, 127, 127, 0.06))',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>Chequeos proactivos automáticos</span>
            <Switch
              on={proactive.enabled}
              onChange={(enabled: boolean) => onChangeProactive({ ...proactive, enabled })}
            />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ fontSize: 12 }} className="muted">Permitir actualizar mini-apps en segundo plano</span>
            <Switch
              on={proactive.appsInBackground}
              onChange={(appsInBackground: boolean) => onChangeProactive({ ...proactive, appsInBackground })}
            />
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 8 }}>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11 }}>
              <span className="muted">Cada (horas)</span>
              <input
                type="number"
                min={1}
                max={24}
                value={proactive.everyHours}
                onChange={(e) => onChangeProactive({ ...proactive, everyHours: Math.max(1, Number(e.target.value) || 4) })}
              />
            </label>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11 }}>
              <span className="muted">Silencio desde (h)</span>
              <input
                type="number"
                min={0}
                max={23}
                value={proactive.quietStart}
                onChange={(e) => onChangeProactive({ ...proactive, quietStart: Math.min(23, Math.max(0, Number(e.target.value) || 0)) })}
              />
            </label>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11 }}>
              <span className="muted">Silencio hasta (h)</span>
              <input
                type="number"
                min={0}
                max={23}
                value={proactive.quietEnd}
                onChange={(e) => onChangeProactive({ ...proactive, quietEnd: Math.min(23, Math.max(0, Number(e.target.value) || 0)) })}
              />
            </label>
          </div>
        </div>
      </div>
    </div>
  )
}
