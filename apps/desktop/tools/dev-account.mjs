// A Clerk test account for `npm run dev` sandboxes: finds or creates `sandbox-<name>+clerk_test@example.com`
// in the Clerk *development* instance and mints a one-time sign-in token the app trades for a session
// (account.ts, `CODYNC_DEV_SIGN_IN_TICKET`). The secret key comes from CLERK_SECRET_KEY or the git-ignored
// `.env` in the main checkout's root (every worktree reads that one file); only `sk_test_` keys are used,
// so production is never touched.
import { spawnSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const API = 'https://api.clerk.com/v1'

export function secretKey() {
  // A linked worktree's git common dir is the main checkout's `.git`, so its parent holds the shared `.env`.
  const common = spawnSync('git', ['rev-parse', '--git-common-dir'], { cwd: dirname(fileURLToPath(import.meta.url)), encoding: 'utf8' }).stdout.trim()
  const file = resolve(dirname(fileURLToPath(import.meta.url)), common, '..', '.env')
  const fromFile = existsSync(file) ? /^CLERK_SECRET_KEY=(.+)$/m.exec(readFileSync(file, 'utf8'))?.[1] : undefined
  const key = (process.env.CLERK_SECRET_KEY ?? fromFile ?? '').trim().replace(/^["']|["']$/g, '')
  if (key && !key.startsWith('sk_test_')) throw new Error('CLERK_SECRET_KEY must be the development instance key (sk_test_...).')
  return key || null
}

async function clerk(key, method, path, body) {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: { Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' },
    body: body && JSON.stringify(body),
    signal: AbortSignal.timeout(20_000),
  })
  const json = await res.json().catch(() => ({}))
  if (!res.ok) throw new Error(json.errors?.[0]?.long_message ?? `Clerk ${method} ${path}: ${res.status}`)
  return json
}

const emailOf = (sandbox) => `sandbox-${sandbox}+clerk_test@example.com`
const findUser = (key, email) => clerk(key, 'GET', `/users?email_address=${encodeURIComponent(email)}`).then((found) => found[0])

/** A fresh sign-in token for the sandbox's test account, or null without a secret key. */
export async function mintTicket(sandbox) {
  const key = secretKey()
  if (!key) return null
  const email = emailOf(sandbox)
  const user = (await findUser(key, email)) ?? (await clerk(key, 'POST', '/users', { email_address: [email], skip_password_requirement: true }))
  const { token } = await clerk(key, 'POST', '/sign_in_tokens', { user_id: user.id, expires_in_seconds: 600 })
  return { token, email }
}

/** Deletes the sandbox's test account; true when there was one. Without a secret key there is nothing to delete. */
export async function deleteTestAccount(sandbox) {
  const key = secretKey()
  const user = key && (await findUser(key, emailOf(sandbox)))
  if (!user) return false
  await clerk(key, 'DELETE', `/users/${user.id}`)
  return true
}
