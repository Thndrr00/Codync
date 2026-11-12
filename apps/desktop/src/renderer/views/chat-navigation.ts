import { useEffect } from 'react'
import type { WindowCommand } from '../../shared/ipc'
import type { IslandAction } from '../../shared/island-activity'
import type { VoiceKey } from '../../shared/voice-shortcut'

interface Navigation {
  newBot(): void
  search(): void
  network(): void
  sidebar(): void
  marketplace(command: Extract<WindowCommand, { kind: 'openMarketplace' }>): void
  voice(key: VoiceKey): void
  island(action: IslandAction): void
  reveal?(ref: string, connectorId?: string): void
}

/** Window navigation stays separate from host mutations and panel rendering. */
export function navigateCommand(command: WindowCommand, navigation: Navigation) {
  switch (command.kind) {
    case 'islandReveal': navigation.reveal?.(command.ref, command.connectorId); break
    case 'newBot': navigation.newBot(); break
    case 'search': navigation.search(); break
    case 'networkSettings': navigation.network(); break
    case 'toggleSidebar': navigation.sidebar(); break
    case 'openMarketplace': navigation.marketplace(command); break
    case 'voiceKey': navigation.voice(command.key); break
    case 'islandAction':
      if (['openFlow', 'openChannels', 'openDiagnostics'].includes(command.action.kind)) navigation.island(command.action)
      break
  }
}

export function useChatNavigation(navigation: Navigation) {
  useEffect(() => window.codync.app.onCommand((command) => navigateCommand(command, navigation)))
}
