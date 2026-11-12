import type { IslandOperationSignals } from '../../shared/island-operations'
import type { UpdateState } from '../../shared/ipc'
import type { SSHState } from '../../shared/ssh'
import { sshStatusDetail } from '../../shared/ssh-startup.ts'
import type { AppModel } from './app-model'

/** No SSH attachments, signing keys, identity paths, fingerprints or SAS codes leave this projection. */
export function islandOperationSignals(app: AppModel, updates: UpdateState | null, ssh: SSHState, updateError: string | null = null): IslandOperationSignals {
  return {
    updateError,
    updates: updates ? { supported: updates.supported, checking: updates.checking, availableVersion: updates.availableVersion, downloadPercent: updates.downloadPercent !== null && Number.isFinite(updates.downloadPercent) ? Math.max(0, Math.min(100, updates.downloadPercent)) : null, staged: updates.staged, installing: updates.installing, error: updates.error } : null,
    ssh: ssh.profiles.map((profile) => {
      const status = ssh.status[profile.id] ?? { kind: 'idle' as const }
      return { id: profile.id, name: profile.name, kind: status.kind, detail: sshStatusDetail(status, ssh.attachments.some((attachment) => attachment.profileId === profile.id)), retryable: ['idle', 'failed', 'retrying'].includes(status.kind) }
    }),
    devices: [
      ...app.approvals.map(({ id, store, request }) => ({ id, computer: store.computer.name, name: request.deviceName, platform: request.platform ?? null, kind: 'access' as const })),
      ...app.pairingAsks.map(({ id, store, request }) => ({ id, computer: store.computer.name, name: request.deviceName, platform: request.platform ?? null, kind: 'pairing' as const })),
    ].slice(0, 30),
  }
}
