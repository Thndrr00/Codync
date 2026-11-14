import { test, expect, type Page } from '@playwright/test'
import { canSteer } from '../src/renderer/views/thread/queued-messages.ts'
import { api, attach, setSteering, steerBot, typeMessage } from './sandbox.ts'

type History = { entries: { kind: string; data: { text?: string; status?: string } }[] }

/** Whether the turn answering `text` has started. */
async function started(page: Page, botId: string, text: string): Promise<boolean> {
  const { entries } = await api<History>(page, 'history', { botId })
  return entries.some((e) => e.kind === 'user' && e.data.text === text && e.data.status === 'sent')
}

test('a long queue scrolls, with its oldest message in reach', async () => {
  const { browser, page } = await attach()
  const name = `Long queue e2e ${Date.now()}`
  let botId: string | undefined
  try {
    // What queues is the subject, so the messages wait for their turn instead of joining one.
    await setSteering(page, false)
    const bot = await steerBot(page, name, 120)
    botId = bot.id
    await page.locator('.roster-row', { hasText: name }).click()
    await api(page, 'send', { botId, text: 'first' })
    await expect.poll(() => started(page, bot.id, 'first')).toBe(true)
    for (let i = 1; i <= 40; i++) await api(page, 'send', { botId, text: `Waiting ${i}` })
    const queued = page.locator('.queued-row')
    await expect(queued).toHaveCount(40)

    await expect(page.locator('.composer textarea')).toBeInViewport()
    const list = page.locator('.queued-list')
    const box = await list.boundingBox()
    expect(box!.y).toBeGreaterThan(0)
    expect(box!.height).toBeLessThan(260)
    const row = (text: string) => queued.filter({ hasText: new RegExp(`^${text}$`) })
    await expect(row('Waiting 1')).toBeInViewport()
    await expect(row('Waiting 40')).not.toBeInViewport()

    await list.hover()
    await page.mouse.wheel(0, 2000)
    await expect(row('Waiting 40')).toBeInViewport()
    await row('Waiting 40').getByRole('button', { name: 'Remove' }).click()
    await expect(queued).toHaveCount(39)
    await page.mouse.wheel(0, -2000)
    await expect(row('Waiting 1')).toBeInViewport()

    // One typed here scrolls into view; the ones the computer got from elsewhere didn't.
    await typeMessage(page, 'typed here')
    await expect(row('typed here')).toBeInViewport()
  } finally {
    if (botId) {
      await api(page, 'stop', { botId }).catch(() => {})
      await api(page, 'deleteBot', { botId }).catch(() => {})
    }
    await setSteering(page, true)
    await browser.close()
  }
})

test('a message sent to a working bot joins its turn by default', async () => {
  const { browser, page } = await attach()
  const name = `Steer e2e ${Date.now()}`
  let botId: string | undefined
  try {
    await setSteering(page, true)
    const { version } = await api<{ version: string }>(page, 'hello', {})
    test.skip(!canSteer(version), `the sandbox host (${version}) can't steer yet`)
    const bot = await steerBot(page, name, 30)
    botId = bot.id
    await page.locator('.roster-row', { hasText: name }).click()
    const transcript = page.locator('.transcript')

    await api(page, 'send', { botId, text: 'first' })
    await expect.poll(() => started(page, bot.id, 'first')).toBe(true)
    // No trip through the queue: it goes straight into the chat and the reply takes it in.
    await typeMessage(page, 'second')
    await expect(transcript).toContainText('second')
    await expect(transcript).toContainText('Got: ')
    // A passed queue is the sending computer's choice, not this app's.
    await expect(page.locator('.queued-row')).toHaveCount(0)
  } finally {
    if (botId) {
      await api(page, 'stop', { botId }).catch(() => {})
      await api(page, 'deleteBot', { botId }).catch(() => {})
    }
    await browser.close()
  }
})

test('a message sent while the bot works waits above the composer, then goes out now or is taken back', async () => {
  const { browser, page } = await attach()
  const name = `Queue e2e ${Date.now()}`
  let botId: string | undefined
  try {
    // Queue chosen: what waits is the subject, and *Send now* steers one by hand.
    await setSteering(page, false)
    const { version } = await api<{ version: string }>(page, 'hello', {})
    test.skip(!canSteer(version), `the sandbox host (${version}) can't steer yet`)
    const bot = await steerBot(page, name, 30)
    botId = bot.id
    await page.locator('.roster-row', { hasText: name }).click()
    const transcript = page.locator('.transcript')
    const queued = page.locator('.queued-row')

    await api(page, 'send', { botId, text: 'first' })
    await expect.poll(() => started(page, bot.id, 'first')).toBe(true)
    await typeMessage(page, 'second')
    await expect(queued.filter({ hasText: 'second' })).toBeVisible()
    await expect(transcript).not.toContainText('second')
    await queued.filter({ hasText: 'second' }).getByRole('button', { name: 'Send now' }).click()
    // It joined the running turn: the chat shows it, and the reply took it in.
    await expect(queued).toHaveCount(0)
    await expect(transcript).toContainText('second')
    await expect(transcript).toContainText('Got: ')

    await api(page, 'send', { botId, text: 'third' })
    await expect.poll(() => started(page, bot.id, 'third')).toBe(true)
    await typeMessage(page, 'never sent')
    await queued.filter({ hasText: 'never sent' }).getByRole('button', { name: 'Remove' }).click()
    await expect(queued).toHaveCount(0)
    await expect(transcript).not.toContainText('never sent')
    const { entries } = await api<History>(page, 'history', { botId })
    expect(entries.find((e) => e.data.text === 'never sent')).toBeUndefined()
  } finally {
    if (botId) {
      await api(page, 'stop', { botId }).catch(() => {})
      await api(page, 'deleteBot', { botId }).catch(() => {})
    }
    await setSteering(page, true)
    await browser.close()
  }
})
