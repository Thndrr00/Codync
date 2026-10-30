import type { AppLinkMode, CreatedAppLink } from '../../shared/bdots-models.ts'
import type { HostClient } from '../client/host-client'

/** Keep the host URL paired with the newly issued capability, never the app owner's URL. */
export function createAppShareLink(client: Pick<HostClient, 'call'>, botId: string, slug: string, mode: AppLinkMode, label: string): Promise<CreatedAppLink> {
  return client.call<CreatedAppLink>('createAppLink', { botId, slug, mode, label })
}
