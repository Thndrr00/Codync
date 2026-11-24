import type { LibraryItem } from '@shared/artifacts'
import { FILE_CHUNK_SIZE, sha256Hex } from '../../../shared/files.ts'
import { trNow } from '../../lib/l10n.ts'
import type { BotStore } from '../../store/bot-store'

/** Saves one selected item with the same authenticated read its chat card uses. */
export async function downloadLibraryItem(store: BotStore, botId: string, item: LibraryItem): Promise<void> {
  const client = store.client
  if (!client) throw new Error(trNow('Connect to this computer to download the file'))
  if (item.kind === 'artifact') {
    await store.artifacts.save(client, store.artifacts.byId.get(item.artifact.id) ?? item.artifact)
    return
  }
  if (item.kind === 'file') {
    // The per-file store shows progress and checks the digest before finishing the save.
    await store.fileDownloads.save(client, item.entryId, item.file)
    return
  }
  // An attachment carries no digest until it is read: readUpload supplies the bytes and we hash them.
  const bytes = await client.readUpload(botId, item.file.id)
  const handle = await window.codync.files.begin({ ...item.file, sha256: await sha256Hex(bytes) })
  if (!handle) return
  try {
    for (let offset = 0; offset < bytes.length; offset += FILE_CHUNK_SIZE) {
      await window.codync.files.write(handle, offset, bytes.subarray(offset, offset + FILE_CHUNK_SIZE))
    }
    await window.codync.files.finish(handle)
  } finally {
    await window.codync.files.cancel(handle).catch(console.error)
  }
}

/**
 * Deletes one selected item with the chat's own deletion call: file messages (and artifact cards) are
 * hidden from the Library as soon as their entry is removed. Artifacts with no card have no such entry.
 */
export async function deleteLibraryItem(store: BotStore, item: LibraryItem): Promise<void> {
  const entryId = item.kind === 'artifact' ? item.artifact.entryId : item.entryId
  if (!entryId) throw new Error(trNow('This artifact has no chat card to delete'))
  const client = store.client
  if (!client) throw new Error(trNow('Connect to this computer to delete the file'))
  await client.call('deleteEntry', { entryId })
}
