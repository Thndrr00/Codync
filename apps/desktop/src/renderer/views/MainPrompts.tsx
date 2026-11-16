import { DeleteAccountPrompt } from './DeleteAccountPrompt'
import { JoinAccountPrompt } from './JoinAccountPrompt'
import { MoveComputerPrompt } from './MoveComputerPrompt'
import { RemoteControlPrompt } from './RemoteControlPrompt'
import { StarPrompt } from './StarPrompt'
import { WhatsNewSheet } from './WhatsNewSheet'

/** What may come up over the chat window once it's `ready` (past the welcome and setup). */
export function MainPrompts({ ready }: { ready: boolean }) {
  return (
    <>
      {ready ? <><StarPrompt /><MoveComputerPrompt /><JoinAccountPrompt /><DeleteAccountPrompt /></> : null}
      {/* Mounted from the start: it decides at mount, before the welcome can finish. */}
      <WhatsNewSheet ready={ready} />
      {/* A paired device asking to control this computer: it can come while the window is anywhere. */}
      <RemoteControlPrompt />
    </>
  )
}
