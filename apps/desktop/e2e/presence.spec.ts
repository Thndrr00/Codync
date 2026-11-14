import { test, expect, type Page } from '@playwright/test'
import { connectPhone } from './phone.ts'
import { api, attach, sandbox } from './sandbox.ts'

// Multiplayer: your other devices are the people of a chat (`presence`, a message's `person`).

/** Who is in the chat right now, as the computer keeps it. */
const presence = (page: Page, botId: string) =>
  api<{ people: { name: string; typing: boolean }[] }>(page, 'presence', { botId })

/** The account the computer is signed in to, if any. */
const owner = async (page: Page) => (await api<{ owner?: { userId: string } | null }>(page, 'cloudStatus', {})).owner?.userId ?? null

test('another paired device shows as someone in the chat, typing, and as the author of what it sends', async () => {
  test.setTimeout(120_000)
  const { browser, page } = await attach()
  const run = Date.now().toString(36)
  const name = `Presence Bot ${run}`
  const phoneName = `E2E phone ${run}`
  let botId: string | undefined
  let phone: Awaited<ReturnType<typeof connectPhone>> | undefined
  try {
    const goOn = page.getByRole('button', { name: /^Continue without an account/ })
    if (await goOn.isVisible({ timeout: 5_000 }).catch(() => false)) await goOn.click()
    test.skip((await owner(page)) !== null, 'Needs a computer in no account: `npm run dev -- --cdp`, without `--cloud`.')

    botId = (await api<{ bot: { id: string } }>(page, 'createBot', {
      name, backend: 'custom', command: 'unused', connectors: [], permission: 'auto', notify: false,
    })).bot.id
    await page.locator('.roster-row', { hasText: name }).click()
    await expect(page.locator('.composer textarea')).toBeVisible()

    // A phone pairs, and the computer tells the person about it first. Anything still asking from
    // an earlier run is turned away, so only this run's request is on screen.
    const dialog = page.getByRole('alertdialog')
    if (await dialog.isVisible({ timeout: 2_000 }).catch(() => false)) {
      await dialog.getByRole('button', { name: "Don't allow" }).click()
    }
    const { pairingUrl } = await api<{ pairingUrl: string }>(page, 'pairing', {})
    const { computerId } = await api<{ computerId: string }>(page, 'hello', {})
    const code = new URL(pairingUrl).searchParams.get('code') as string
    const pairing = connectPhone(sandbox().port, computerId, code, phoneName)
    await expect(dialog.getByRole('heading', { name: `Let ${phoneName} in?` })).toBeVisible({ timeout: 20_000 })
    await dialog.getByRole('button', { name: 'Let in' }).click()
    phone = await pairing

    // The phone looks at the chat and keeps typing: the row names it.
    const row = page.locator('.presence-row')
    for (let i = 0; i < 4; i++) {
      await phone.call('presence', { botId, typing: true })
      await page.waitForTimeout(800)
    }
    await expect(row).toContainText(phoneName)
    await expect(row).toContainText('is typing')
    // The computer keeps the phone among the people of this chat, and only of this one.
    expect((await presence(page, botId)).people.map((person) => person.name)).toContain(phoneName)

    // What it sends carries its author, and the chat shows who wrote it.
    const text = `From the phone ${run}`
    await phone.call('send', { botId, text, clientNonce: crypto.randomUUID() })
    const said = page.locator('.chat-row', { hasText: text })
    await expect(said).toBeVisible()
    await expect(said.locator('.chat-person')).toHaveText(phoneName)

    // Typing goes quiet on its own: a paired device stays "here" for half a minute instead.
    await expect(row).toContainText('is here', { timeout: 15_000 })
  } finally {
    phone?.close()
    const key = (await api<{ devices: { name: string; key: string }[] }>(page, 'devices', {}).catch(() => ({ devices: [] }))).devices
      .find((device) => device.name === phoneName)?.key
    if (key) await api(page, 'revokeDevice', { key }).catch(() => {})
    if (botId) await api(page, 'deleteBot', { botId }).catch(() => {})
    await browser.close()
  }
})
