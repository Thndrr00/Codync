import { test, expect } from '@playwright/test'
import { api, attach } from './sandbox.ts'

test('a message pins from its menu into the chat\'s pin list, and unpins from there', async () => {
  const { browser, page } = await attach()
  const run = Date.now().toString(36)
  const name = `Pin Bot ${run}`
  const text = `Keep this around ${run}`
  let botId: string | undefined
  const pins = async () => (await api<{ pins: { text: string }[] }>(page, 'pinnedMessages', { botId })).pins
  try {
    // The pin copy is asserted in English, whatever the computer's own language.
    await page.evaluate(() => localStorage.setItem('appLanguage', JSON.stringify('en')))
    await page.reload()
    // A fresh sandbox still asks for an account first.
    const goOn = page.getByRole('button', { name: /^Continue without an account/ })
    if (await goOn.isVisible({ timeout: 5_000 }).catch(() => false)) await goOn.click()
    // Getting started walks through the host and the phone; it covers the roster.
    const notNow = page.getByRole('dialog').getByRole('button', { name: 'Not now' })
    if (await notNow.isVisible({ timeout: 5_000 }).catch(() => false)) await notNow.click()

    botId = (await api<{ bot: { id: string } }>(page, 'createBot', { name, backend: 'custom', command: 'unused', connectors: [], permission: 'auto', notify: false })).bot.id
    await api(page, 'send', { botId, text })
    await page.locator('.roster-row', { hasText: name }).click()
    const message = page.locator('.message.user', { hasText: text })
    await expect(message).toBeVisible()

    // The header button opens an empty list first.
    await page.getByRole('button', { name: 'Pinned messages' }).click()
    await expect(page.getByText("Nothing is pinned here yet. Open a message's menu and pick Pin.")).toBeVisible()
    await page.getByRole('button', { name: 'Close' }).click()

    // Right-click, then Pin: the menu offers the opposite afterwards.
    await message.locator('.bubble').click({ button: 'right' })
    await page.getByRole('menuitem', { name: 'Pin', exact: true }).click()
    await expect.poll(async () => (await pins()).map((pin) => pin.text)).toEqual([text])
    await message.locator('.bubble').click({ button: 'right' })
    await expect(page.getByRole('menuitem', { name: 'Unpin', exact: true })).toBeVisible()
    await page.keyboard.press('Escape')

    // The list names who pinned it and what it says, and takes the pin back.
    await page.getByRole('button', { name: 'Pinned messages' }).click()
    const row = page.locator('.pin-row', { hasText: text })
    await expect(row).toContainText(`You · ${name}`)
    await row.getByRole('button', { name: 'Unpin' }).click()
    await expect(page.getByText("Nothing is pinned here yet. Open a message's menu and pick Pin.")).toBeVisible()
    await expect.poll(pins).toHaveLength(0)
  } finally {
    if (botId) await api(page, 'deleteBot', { botId }).catch(() => {})
    await page.evaluate(() => localStorage.removeItem('appLanguage')).catch(() => {})
    await browser.close()
  }
})
