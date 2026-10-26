import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createAppShareLink } from './app-links.ts'
import type { HostClient } from '../client/host-client'

for (const mode of ['view', 'edit'] as const) {
  test(`sharing preserves the host URL for the newly issued ${mode} capability`, async () => {
    const requests: unknown[] = []
    const token = `new-${mode}-token`
    const response = {
      link: { token, botId: 'bot', slug: 'tracker', label: 'Team', mode, createdAt: 1, opens: 0 },
      url: `http://127.0.0.1:54123/apps/bot/tracker/${token}/`,
    }
    const client = { call: async (method: string, body: unknown) => {
      requests.push({ method, body })
      return response
    } } as unknown as Pick<HostClient, 'call'>
    const shared = await createAppShareLink(client, 'bot', 'tracker', mode, 'Team')
    assert.deepEqual(requests, [{ method: 'createAppLink', body: { botId: 'bot', slug: 'tracker', mode, label: 'Team' } }])
    assert.equal(shared.url, response.url)
    assert.equal(new URL(shared.url).pathname, `/apps/bot/tracker/${shared.link.token}/`)
    assert.equal(shared.link.mode, mode)
  })
}
