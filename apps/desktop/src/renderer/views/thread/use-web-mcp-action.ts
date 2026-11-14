// A host-served mini-app page asked Codync to show one of its granted panels (card K15). The chat
// window brings the bot on screen; this hook finishes the action once that chat is mounted.

import { useEffect } from 'react'
import type { BotStore } from '../../store/bot-store'

export function useWebMcpAction(
  store: BotStore,
  botId: string,
  handlers: { onArtifact: (id: string) => void; onCall: () => void },
) {
  const action = store.lastWebMcpAction
  const { onArtifact, onCall } = handlers
  useEffect(() => {
    if (!action) return
    const taken = store.takeWebMcpAction(botId)
    if (!taken) return
    if (taken.action === 'show_artifact') {
      const artifactId = taken.arguments.artifactId
      if (typeof artifactId === 'string') onArtifact(artifactId)
    } else if (taken.action === 'start_call') {
      onCall()
    }
  }, [action, botId, store, onArtifact, onCall])
}
