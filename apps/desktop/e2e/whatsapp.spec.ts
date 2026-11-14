import { test, expect } from '@playwright/test'
import { api, attach } from './sandbox.ts'

type State = { workingMessage?: boolean }

test('the WhatsApp working message is a switch the host stores and reads back', async () => {
  // No phone is linked in a sandbox, so the sheet cannot draw its switch yet; the host path it drives is
  // what runs here, from the same methods the sheet calls.
  const { browser, page } = await attach()
  const state = async () => (await api<State>(page, 'whatsappState', {})).workingMessage
  const before = await state()
  try {
    await api(page, 'whatsappSetWorkingMessage', { enabled: false })
    await expect.poll(state).toBe(false)
    await api(page, 'whatsappSetWorkingMessage', { enabled: true })
    await expect.poll(state).toBe(true)
  } finally {
    await api(page, 'whatsappSetWorkingMessage', { enabled: before ?? true }).catch(() => {})
    await browser.close()
  }
})
