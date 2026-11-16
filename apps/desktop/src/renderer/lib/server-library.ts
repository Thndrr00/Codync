import type { ServerLibraryFilter, ServerLibraryItem } from '@shared/artifacts'

/**
 * What a server's Library shows for `filter`, newest first: the same rule the host applies (`image` is true for the
 * extensions the apps preview), so switching the filter never needs another read.
 */
export function serverLibraryItems(items: ServerLibraryItem[], filter: ServerLibraryFilter): ServerLibraryItem[] {
  if (filter === 'all') return items
  return items.filter((item) => (filter === 'images' ? item.image : !item.image))
}
