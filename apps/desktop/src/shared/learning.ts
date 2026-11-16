/** Facts a bot proposes from the person's corrections and repeated instructions (`learning`): they wait for review. */

export type LearningKind = 'correction' | 'instruction' | 'repetition'

export interface LearningProposal {
  id: string
  botId: string
  text: string
  kind: LearningKind
  /** How many times the person said it. */
  hits: number
  /** The message it came from, cut short. */
  source: string
  createdAt: number
}

export interface LearningInbox {
  enabled: boolean
  autoApproveAfter: number
  maxAutoApproveAfter: number
  note: string
  proposals: LearningProposal[]
}

export const learnedTitle = (count: number) => (count > 0 ? `Learned preferences (${count})` : 'Learned preferences')

export const kindLabel = (kind: LearningKind) => kind === 'correction' ? 'Correction' : kind === 'repetition' ? 'Said again' : 'Preference'

/** The repeat count a card shows: `Said again · 3 times`; a correction says nothing about repeats. */
export const hitsLabel = (proposal: LearningProposal) =>
  proposal.kind === 'correction' ? 'Correction' : `${kindLabel(proposal.kind)}${proposal.hits > 1 ? ` · ${proposal.hits} times` : ''}`
