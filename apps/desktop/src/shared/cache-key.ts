// One cache entry's file name: a hash of the key, so no account or computer name lands on disk.
// FNV-1a twice, with different offsets, keeps the names short and collision-resistant enough for
// a handful of computers per account.

function hex(value: number): string {
  return (value >>> 0).toString(16).padStart(8, '0')
}

export function cacheFileName(key: string): string {
  let first = 0x811c9dc5
  let second = 0x7fffffff
  for (let i = 0; i < key.length; i++) {
    const code = key.charCodeAt(i)
    first = Math.imul(first ^ code, 0x01000193)
    second = Math.imul(second ^ code, 0x85ebca6b) ^ (second >>> 13)
  }
  return `${hex(first)}${hex(second)}.bin`
}
