import { test, expect } from '@playwright/test'
import { api, attach } from './sandbox.ts'

type Stored = { bot: { notify: boolean | null } }

test('the bot menu mutes an agent, and its notification switch gives the alerts back', async () => {
  test.setTimeout(90_000)
  const { browser, page } = await attach()
  const name = `Mute Bot ${Date.now().toString(36)}`
  let botId: string | undefined
  // What the host stored: the switch is checked where it lives, not only as the menu draws it.
  const stored = async () => (await api<Stored>(page, 'updateBot', { id: botId })).bot.notify
  try {
    // The menu copy is asserted in English, whatever the computer's own language.
    await page.evaluate(() => localStorage.setItem('appLanguage', JSON.stringify('en')))
    await page.reload()
    // A fresh sandbox still asks for an account first.
    const goOn = page.getByRole('button', { name: /^Continue without an account/ })
    if (await goOn.isVisible({ timeout: 5_000 }).catch(() => false)) await goOn.click()
    // Getting started walks through the host and the phone; it covers the roster.
    const notNow = page.getByRole('dialog').getByRole('button', { name: 'Not now' })
    if (await notNow.isVisible({ timeout: 5_000 }).catch(() => false)) await notNow.click()

    botId = (await api<{ bot: { id: string } }>(page, 'createBot', {
      name, backend: 'custom', command: 'unused', connectors: [], permission: 'auto',
    })).bot.id
    // Unset, the bot takes the computer's own choice.
    expect(await stored()).toBeNull()

    const row = page.locator('.roster-row', { hasText: name })
    await row.click()
    // Right-click, then Mute: the menu offers the opposite afterwards.
    await row.click({ button: 'right' })
    await page.getByRole('menuitem', { name: 'Mute agent', exact: true }).click()
    await expect.poll(stored).toBe(false)
    await row.click({ button: 'right' })
    await expect(page.getByRole('menuitem', { name: 'Unmute agent', exact: true })).toBeVisible()
    await page.keyboard.press('Escape')

    // The agent form shows the muted bot, and its switch gives the alerts back.
    await row.click({ button: 'right' })
    await page.getByRole('menuitem', { name: /^Edit Profile/ }).click()
    const sheet = page.getByRole('dialog')
    const notifications = sheet.getByRole('switch', { name: 'Notifications' })
    await expect(notifications).toHaveAttribute('aria-checked', 'false')
    await notifications.click()
    await sheet.getByRole('button', { name: 'Save', exact: true }).click()
    // An on switch stores an explicit true; the alert gate treats that like unset, either way not muted.
    await expect.poll(stored).toBe(true)
  } finally {
    if (botId) await api(page, 'deleteBot', { botId }).catch(() => {})
    await page.evaluate(() => localStorage.removeItem('appLanguage')).catch(() => {})
    await browser.close()
  }
})
