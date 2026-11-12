import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { UpdateState } from '../../../shared/ipc.ts'
import type { SSHState, SSHStatus } from '../../../shared/ssh.ts'
import type { AppModel } from '../../store/app-model.ts'
import { islandOperationSignals } from '../../store/island-operation-signals.ts'

const emptyApp = { approvals: [], pairingAsks: [] } as unknown as AppModel
const emptySSH: SSHState = { profiles: [], status: {}, attachments: [], needsLinger: [] }
const update = (downloadPercent: number | null): UpdateState => ({ supported: true, checking: false, availableVersion: '2.44.2', downloadPercent, staged: false, installing: false, autoCheck: true, autoDownload: true, lastCheck: 100, error: null })

test('update progress preserves real zero, clamps finite bounds and leaves invalid progress unknown', () => {
  for (const [value, expected] of [[0, 0], [1.5, 1.5], [70, 70], [100, 100], [-5, 0], [105, 100], [null, null], [NaN, null], [Infinity, null], [-Infinity, null]]) {
    assert.equal(islandOperationSignals(emptyApp, update(value), emptySSH).updates?.downloadPercent, expected, String(value))
  }
  assert.equal(islandOperationSignals(emptyApp, null, emptySSH).updates, null)
  const missing = { ...update(null), downloadPercent: undefined } as unknown as UpdateState
  assert.equal(islandOperationSignals(emptyApp, missing, emptySSH).updates?.downloadPercent, null)
})

test('update signals include only display state and preserve a failed state read without initiating an update', () => {
  const source = { ...update(0), checking: true, staged: true, installing: true, error: 'Update unavailable', url: 'PRIVATE_UPDATE_URL', token: 'PRIVATE_UPDATE_TOKEN', headers: { Authorization: 'PRIVATE_AUTHORIZATION' } }
  const snapshot = islandOperationSignals(emptyApp, source, emptySSH)
  assert.deepEqual(snapshot.updates, { supported: true, checking: true, availableVersion: '2.44.2', downloadPercent: 0, staged: true, installing: true, error: 'Update unavailable' })
  assert.equal(JSON.stringify(snapshot).includes('PRIVATE'), false)
  assert.equal(snapshot.updateError, null)
  const failed = islandOperationSignals(emptyApp, null, emptySSH, 'Could not read update state')
  assert.equal(failed.updates, null)
  assert.equal(failed.updateError, 'Could not read update state')
})

function sshWithStatuses(statuses: Record<string, SSHStatus>): SSHState {
  return {
    profiles: Object.keys(statuses).map((id) => ({ id, name: `Computer ${id}`, host: 'PRIVATE_HOST', port: 22, user: 'PRIVATE_USER', identityFile: 'PRIVATE_KEY_PATH', remotePort: 19222, computerId: `pc-${id}` })),
    status: { ...statuses, orphan: { kind: 'failed', message: 'PRIVATE_ORPHAN_STATUS' } },
    attachments: [{ profileId: 'connected', computer: { id: 'pc-connected', name: 'Computer connected', signKey: 'PRIVATE_SIGN_KEY', boxKey: 'PRIVATE_BOX_KEY' }, baseURL: 'PRIVATE_LOOPBACK_URL', token: 'PRIVATE_ATTACHMENT_TOKEN', vncPort: 19333 }],
    needsLinger: ['PRIVATE_LINGER_PROFILE'],
  }
}

test('SSH signals expose known profile names and classified status without tunnel credentials or consent material', () => {
  const ssh = sshWithStatuses({
    idle: { kind: 'idle' },
    failed: { kind: 'failed', message: 'Could not connect' },
    retrying: { kind: 'retrying', message: 'Connection lost' },
    confirm: { kind: 'confirmHostKey', name: 'PRIVATE_RAW_NAME', fingerprints: ['PRIVATE_FINGERPRINT'] },
    missing: { kind: 'notInstalled', failure: { output: 'PRIVATE_INSTALL_LOG' } },
    installing: { kind: 'installing', step: 'Installing the host' },
    connecting: { kind: 'connecting', step: 'Opening the tunnel' },
    connected: { kind: 'connected', localPort: 19333 },
    opening: { kind: 'connected', localPort: 19444 },
  })
  const snapshot = islandOperationSignals(emptyApp, null, ssh)
  assert.equal(snapshot.ssh.length, 9)
  assert.equal(snapshot.ssh.some((profile) => profile.id === 'orphan'), false)
  assert.equal(JSON.stringify(snapshot).includes('PRIVATE'), false)
  const byId = new Map(snapshot.ssh.map((profile) => [profile.id, profile]))
  assert.equal(byId.get('failed')?.detail, 'Could not connect')
  assert.equal(byId.get('retrying')?.detail, 'Connection lost Retrying\u2026')
  assert.equal(byId.get('confirm')?.detail, 'Confirm host key\u2026')
  assert.equal(byId.get('missing')?.detail, 'Host not installed')
  assert.equal(byId.get('connected')?.detail, 'Connected')
  assert.equal(byId.get('opening')?.detail, 'Connected \u00b7 opening\u2026')
  assert.deepEqual(snapshot.ssh.filter((profile) => profile.retryable).map((profile) => profile.id), ['idle', 'failed', 'retrying'])
  assert.deepEqual(Object.keys(snapshot.ssh[0]).sort(), ['detail', 'id', 'kind', 'name', 'retryable'])
})

test('a saved SSH profile with no status remains idle and retryable without inventing a tunnel', () => {
  const ssh = sshWithStatuses({ saved: { kind: 'idle' } })
  delete ssh.status.saved
  const snapshot = islandOperationSignals(emptyApp, null, ssh)
  assert.deepEqual(snapshot.ssh, [{ id: 'saved', name: 'Computer saved', kind: 'idle', detail: 'Not connected', retryable: true }])
})

test('device signals retain request display identities while excluding SAS codes, email and device keys', () => {
  const store = { computer: { id: 'computer', name: 'Workstation', signKey: 'PRIVATE_COMPUTER_KEY' }, client: { token: 'PRIVATE_CLIENT_TOKEN' } }
  const app = {
    approvals: [{ id: 'computer/access-1', store, request: { requestId: 'access-1', deviceName: 'Phone', platform: 'ios', code: 'PRIVATE_ACCESS_CODE', deviceKey: 'PRIVATE_DEVICE_KEY', email: 'PRIVATE_EMAIL', expiresAt: 100 } }],
    pairingAsks: [{ id: 'computer/pairing-1', store, request: { requestId: 'pairing-1', deviceName: 'Tablet', code: 'PRIVATE_PAIRING_CODE', email: 'PRIVATE_PAIRING_EMAIL', deviceKey: 'PRIVATE_EXTRA_KEY' } }],
  } as unknown as AppModel
  const snapshot = islandOperationSignals(app, null, emptySSH)
  assert.deepEqual(snapshot.devices, [
    { id: 'computer/access-1', computer: 'Workstation', name: 'Phone', platform: 'ios', kind: 'access' },
    { id: 'computer/pairing-1', computer: 'Workstation', name: 'Tablet', platform: null, kind: 'pairing' },
  ])
  assert.equal(JSON.stringify(snapshot).includes('PRIVATE'), false)
})

test('pending device metadata is capped without exposing the remaining requests or their codes', () => {
  const store = { computer: { name: 'Workstation' } }
  const requests = Array.from({ length: 40 }, (_, index) => ({ id: `computer/${index}`, store, request: { deviceName: `Device ${index}`, platform: 'android', code: `PRIVATE_CODE_${index}`, deviceKey: 'PRIVATE_KEY' } }))
  const app = { approvals: requests.slice(0, 20), pairingAsks: requests.slice(20) } as unknown as AppModel
  const snapshot = islandOperationSignals(app, null, emptySSH)
  assert.equal(snapshot.devices.length, 30)
  assert.equal(snapshot.devices[19].kind, 'access')
  assert.equal(snapshot.devices[20].kind, 'pairing')
  assert.equal(snapshot.devices[29].id, 'computer/29')
  assert.equal(snapshot.devices.some((request) => request.id === 'computer/30'), false)
  assert.equal(JSON.stringify(snapshot).includes('PRIVATE'), false)
})
