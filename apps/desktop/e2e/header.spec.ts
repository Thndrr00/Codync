import { test, expect } from '@playwright/test'
import { api, attach } from './sandbox.ts'

test("the chat header shows the agent's purpose after its name, and a narrow one keeps the name", async () => {
  // The first window of a run has the onboarding between it and the roster, which can outlast the default.
  test.setTimeout(90_000)
  const { browser, page } = await attach()
  const run = Date.now().toString(36)
  const name = `Header Bot ${run}`
  const purpose = `Reviews every pull request ${run}`
  try {
    // The copy is asserted in English, whatever the computer's own language.
    await page.evaluate(() => localStorage.setItem('appLanguage', JSON.stringify('en')))
    await page.reload()
    // A fresh sandbox still asks for an account first.
    const goOn = page.getByRole('button', { name: /^Continue without an account/ })
    if (await goOn.isVisible({ timeout: 5_000 }).catch(() => false)) await goOn.click()
    // Getting started walks through the host and the phone; it covers the roster.
    const notNow = page.getByRole('dialog').getByRole('button', { name: 'Not now' })
    if (await notNow.isVisible({ timeout: 5_000 }).catch(() => false)) await notNow.click()

    await api(page, 'createBot', { name, description: purpose, backend: 'custom', command: 'unused', connectors: [], permission: 'auto', notify: false })
    await page.locator('.roster-row', { hasText: name }).click()

    const pill = page.locator('.title-pill')
    await expect(pill).toContainText(name)
    await expect(pill).toContainText(purpose)

    // A narrow window hides the purpose and keeps the name: what 0.70.0 asks of the header.
    await page.setViewportSize({ width: 620, height: 820 })
    await expect(pill).not.toContainText(purpose)
    await expect(pill).toContainText(name)
  } finally {
    await browser.close()
  }
})
