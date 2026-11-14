import { test, expect } from '@playwright/test'
import { api, attach } from './sandbox.ts'

test('/image opens the picker, and using it writes the command the host reads', async () => {
  const { browser, page } = await attach()
  const name = `Media Bot ${Date.now().toString(36)}`
  let botId: string | undefined
  const field = page.locator('.composer textarea')
  const sheet = page.getByRole('dialog')
  try {
    // The sheet's copy is asserted in English, whatever the computer's own language.
    await page.evaluate(() => localStorage.setItem('appLanguage', JSON.stringify('en')))
    await page.reload()
    // A fresh sandbox still asks for an account first.
    const goOn = page.getByRole('button', { name: /^Continue without an account/ })
    if (await goOn.isVisible({ timeout: 5_000 }).catch(() => false)) await goOn.click()
    const notNow = page.getByRole('dialog').getByRole('button', { name: 'Not now' })
    if (await notNow.isVisible({ timeout: 5_000 }).catch(() => false)) await notNow.click()

    // Codex is the agent whose subscription pays for a picture.
    const { bot } = await api<{ bot: { id: string } }>(page, 'createBot', { name, backend: 'codex', connectors: [], permission: 'auto', notify: false })
    botId = bot.id
    await page.locator('.roster-row', { hasText: name }).click()

    // The command in the list opens the picker, which names who makes the file.
    await field.fill('/image')
    await page.getByRole('option', { name: /^Image/ }).click()
    await expect(sheet.getByText('Made by codex, paid with its subscription')).toBeVisible()
    // Only the settings that model has: a Codex picture takes no length, and can be clear.
    await expect(sheet.getByText('Aspect ratio')).toBeVisible()
    await expect(sheet.getByText('Length')).toHaveCount(0)
    await expect(sheet.getByRole('switch', { name: 'Transparent background' })).toBeVisible()

    // A Codex bot makes no clip; the switch says so instead of offering one.
    await sheet.getByRole('button', { name: 'Video', exact: true }).click()
    await expect(sheet.getByText("This bot's agent makes no video.")).toBeVisible()
    await sheet.getByRole('button', { name: 'Image', exact: true }).click()

    // The preview is the message the host parses, and each choice shows in it.
    await sheet.getByRole('button', { name: '16:9' }).click()
    await expect(sheet.locator('.media-preview')).toHaveText('/image --model gpt-image --aspect 16:9')
    await sheet.getByRole('switch', { name: 'Transparent background' }).click()
    await expect(sheet.locator('.media-preview')).toHaveText('/image --model gpt-image --aspect 16:9 --transparent')

    // Use writes it into the box, ready for a description, and closes the sheet.
    await sheet.getByRole('button', { name: 'Use' }).click()
    await expect(sheet).toHaveCount(0)
    await expect(field).toHaveValue('/image --model gpt-image --aspect 16:9 --transparent ')
  } finally {
    if (botId) await api(page, 'deleteBot', { botId }).catch(() => {})
    await page.evaluate(() => localStorage.removeItem('appLanguage')).catch(() => {})
    await browser.close()
  }
})
