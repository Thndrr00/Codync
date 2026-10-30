import {
  BOT_AVATAR_LOOKS,
  WORKER_ROLE_TEMPLATES,
  type BotAvatarLook,
  type WorkerRoleTemplate,
} from '@shared/models'
import { avatarColor } from '../lib/theme'
import { RobotAvatar } from './RobotAvatar'

export interface ProactivePolicyProps {
  avatarColorId: string
  avatarLook?: string
  onChangeLook: (look: string | undefined) => void
  onApplyRoleTemplate?: (template: WorkerRoleTemplate) => void
}

/** Role instructions and avatar choices supported by the host. */
export function ProactivePolicySection({
  avatarColorId,
  avatarLook,
  onChangeLook,
  onApplyRoleTemplate,
}: ProactivePolicyProps) {
  const colorHex = avatarColor(avatarColorId)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      {onApplyRoleTemplate ? (
        <div>
          <div className="eyebrow" style={{ marginBottom: 8 }}>Worker role profiles</div>
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
    </div>
  )
}
