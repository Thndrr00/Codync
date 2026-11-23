/** A place the bot can answer in: the linked number's own chat first, then the allowed chats. */
export interface WhatsAppPlace {
  id: string
  own: boolean
}

/**
 * The owner's "Message yourself" chat always reaches the bot, even on a number
 * used only for OpenBot and with a non-empty allowlist (OpenBot triage-13865).
 * It is therefore a place in the list and is never one of the removable rows.
 */
export function whatsappPlaces(me: string | null | undefined, allowed: string[]): WhatsAppPlace[] {
  const own = me && me.trim() ? [{ id: me, own: true }] : []
  return [...own, ...allowed.filter((id) => id !== me).map((id) => ({ id, own: false }))]
}
