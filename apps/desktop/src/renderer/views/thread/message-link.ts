/**
 * The stable link to one message: `bot:<bot id>/entry:<entry id>`. The host owns this format in
 * `api/message_link.rs` (`messageLink`) so every client copies the same opaque string; a message row
 * already has both ids in the live store, so it can copy the link without a round trip.
 */
export function messageLink(botId: string, entryId: string): string {
  return `bot:${botId}/entry:${entryId}`
}
