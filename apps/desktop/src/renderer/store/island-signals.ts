import type { IslandSignals } from '../../shared/island-integrations'
import type { AppModel } from './app-model'
import { activeDictation } from '../views/thread/active-dictation.ts'

/** Existing device state, projected without audio, message bodies or file bytes. */
export function islandSignals(app: AppModel): IslandSignals {
  return {
    computers: [...app.stores.values()].map((store) => ({ id: store.computer.id, name: store.computer.name, state: store.connection.kind, error: store.lastError })),
    uploads: app.roster.flatMap(({ bot, ref, store }) => store.allEntries(bot.id).filter((entry) => entry.kind === 'user' && entry.data.status === 'sending').flatMap((entry) => (entry.data.attachments ?? []).map((file) => ({ ref: `${ref.computerId}/${ref.botId}`, name: file.name.slice(0, 180), percent: store.uploadPercents.get(file.id) ?? null, status: store.uploadPercents.has(file.id) ? 'Uploading' : 'Sending' })))).slice(0, 20),
    dictation: activeDictation.snapshot,
  }
}
