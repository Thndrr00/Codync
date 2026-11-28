/**
 * With Sort chats = Manual, dropping a chat at the top of its own folder keeps it in that
 * folder: the drop target is the folder it came from, at a bounded insertion index. A missing
 * folder is the root; negative or fractional positions become the top slot.
 */
export type ManualDropTarget = { folder: string; position: number }

export function manual_drop_target(folder: string, position: number): ManualDropTarget {
  const name = (folder ?? '').trim()
  const at = Number.isFinite(position) ? Math.max(0, Math.floor(position)) : 0
  return { folder: name, position: at }
}
