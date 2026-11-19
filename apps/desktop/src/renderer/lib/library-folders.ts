import type { LibraryItem } from '@shared/artifacts'
import { isRecent } from './library-groups.ts'

/** A chat's Library folder as the host lists it (`libraryFolders`). */
export interface LibraryFolder {
  id: string
  name: string
  at: number
  count: number
  items: string[]
}

/** The side rail's built-in views; any other value is a folder id. */
export type LibrarySection = 'all' | 'recent' | 'unfiled' | 'favorites' | string

/** The key both ends use to name an item when it is filed in a folder. */
export function libraryItemKey(item: LibraryItem): string {
  if (item.kind === 'artifact') return `artifact:${item.artifact.id}`
  return `${item.kind}:${item.entryId}:${item.file.id}`
}

/** The item name shown on a folder tile's newest list. */
export function libraryItemName(item: LibraryItem): string {
  return item.kind === 'artifact' ? item.artifact.title : item.file.name
}

/** Whether an item belongs in a side-rail view. Favorites are the items in starred folders, on this device only. */
export function inSection(item: LibraryItem, section: LibrarySection, favorites: readonly string[], now: number): boolean {
  switch (section) {
    case 'all': return true
    case 'recent': return isRecent(item.at, now)
    case 'unfiled': return !item.folderId
    case 'favorites': return !!item.folderId && favorites.includes(item.folderId)
    default: return item.folderId === section
  }
}

/** The folders shown above the files in a view: starred ones in Favorites, otherwise the chat's folders. */
export function visibleFolders(folders: readonly LibraryFolder[], section: LibrarySection, favorites: readonly string[]): LibraryFolder[] {
  if (section === 'favorites') return folders.filter((folder) => favorites.includes(folder.id))
  if (section === 'all' || section === 'recent') return [...folders]
  if (section === 'unfiled') return []
  return folders.filter((folder) => folder.id === section)
}

function favoritesKey(botId: string): string {
  return `codync.library.favorites.${botId}`
}

/** Starred folder ids of one chat, kept in this browser only. A corrupt or older value reads as nothing starred. */
export function readFavorites(botId: string): string[] {
  try {
    const raw = localStorage.getItem(favoritesKey(botId))
    const ids: unknown = raw ? JSON.parse(raw) : []
    return Array.isArray(ids) ? ids.filter((id): id is string => typeof id === 'string') : []
  } catch {
    return []
  }
}

/** Stars or unstars one folder and returns the new list. */
export function toggleFavorite(botId: string, folderId: string): string[] {
  const ids = readFavorites(botId)
  const next = ids.includes(folderId) ? ids.filter((id) => id !== folderId) : [...ids, folderId]
  try {
    localStorage.setItem(favoritesKey(botId), JSON.stringify(next))
  } catch {
    // Storage can be unavailable (private mode); the change still applies until this view closes.
  }
  return next
}
