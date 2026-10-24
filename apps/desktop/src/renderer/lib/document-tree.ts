export interface PageNode { id: string; parentId: string | null; title: string }

/** Missing ancestors remain visible until further metadata pages load. */
export function documentTree<T extends PageNode>(items: T[], searching = false) {
  const output: { page: T; depth: number }[] = []
  const seen = new Set<string>()
  const ids = new Set(items.map((item) => item.id))
  const children = new Map<string | null, T[]>()
  for (const item of items) {
    const parent = searching || !item.parentId || !ids.has(item.parentId) ? null : item.parentId
    const siblings = children.get(parent) ?? []
    siblings.push(item); children.set(parent, siblings)
  }
  function visit(page: T, depth: number) {
    if (seen.has(page.id)) return
    seen.add(page.id); output.push({ page, depth })
    for (const child of children.get(page.id) ?? []) visit(child, depth + 1)
  }
  for (const page of children.get(null) ?? []) visit(page, 0)
  for (const page of items) if (!seen.has(page.id)) visit(page, 0)
  return output
}

export function documentDescendants(items: PageNode[], id: string): Set<string> {
  const found = new Set([id])
  let changed = true
  while (changed) {
    changed = false
    for (const item of items) if (item.parentId && found.has(item.parentId) && !found.has(item.id)) { found.add(item.id); changed = true }
  }
  return found
}
