import { useState } from 'react'
import { ModalHeader, Sheet } from '../components/Overlay'
import { Icon } from '../components/Icon'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { Field } from './marketplace/pieces'
import './prompt-library.css'

interface Prompt {
  title: string
  detail: string
  icon: string
  text: string
}

/**
 * A starting point for a message. Each one is a short, complete instruction with the blanks
 * marked in square brackets; picking one fills the composer, where it can be edited before sending.
 * Nothing here runs on its own or reaches a model until the person sends it.
 */
const prompts: Prompt[] = [
  {
    title: 'Summarize this',
    detail: 'A short summary, the key points, and what to do next.',
    icon: 'doc.plaintext',
    text: 'Summarize the following, in this order: one sentence saying what it is, the key points as a short list, and any decision or next step it implies. Say what is missing if something needed is not there.\n\n[paste the text or attach the file]',
  },
  {
    title: 'Plan a project',
    detail: 'Goal, steps, order, risks and what done means.',
    icon: 'list.bullet.rectangle',
    text: 'Plan this project: [what it is]. Give me the goal in one sentence, the steps in the order they should happen with the result of each, the risks with how I would notice them, and a clear test for what "done" means. Ask me one question first only if the answer would change the plan.',
  },
  {
    title: 'Review a change',
    detail: 'Find what breaks, with the smallest fix for each.',
    icon: 'checklist',
    text: 'Review this change as a careful reviewer. For each problem, give the trigger (an input or state), the wrong result, and the smallest fix. Separate blockers from nits. If you cannot name a trigger, leave it out.\n\n[paste the diff or point me at the files]',
  },
  {
    title: 'Find the risks',
    detail: 'What can go wrong, how I would see it, and what to do.',
    icon: 'exclamationmark.triangle',
    text: 'List the risks in [the plan, change or situation]. For each one: what would trigger it, what the consequence is, how I would notice it early, and the cheapest response. Rank them by how much they would hurt, not by how likely they feel.',
  },
  {
    title: 'Notes into actions',
    detail: 'Decisions, owners, dates and open questions.',
    icon: 'checkmark.circle',
    text: 'Turn these notes into decisions, actions and open questions. Each action gets an owner, a date if the notes give one, and the line it came from. Do not invent an owner or a date; write "no owner" or "no date" instead.\n\n[paste the notes]',
  },
  {
    title: 'Draft an email',
    detail: 'One message with the reason, the ask and the next step.',
    icon: 'square.and.pencil',
    text: 'Draft an email to [who] about [what]. Put the reason for writing in the first sentence, then what I need from them as one clear ask, then what happens next. Keep it under 150 words and match a plain, direct voice. Do not invent facts, prices or promises.',
  },
  {
    title: 'Compare two options',
    detail: 'Criteria from my decision, not from the products.',
    icon: 'arrow.left.arrow.right',
    text: 'Compare [option A] and [option B] for [the decision I am making]. First list the criteria that matter for this decision and mark which are must-haves. Then fill a small table with a source or a reason per cell. End with which one fits and what would change the answer.',
  },
  {
    title: 'Debug this',
    detail: 'Reproduce, find the first wrong step, then the smallest fix.',
    icon: 'wrench.and.screwdriver',
    text: 'Help me debug this. Here is what I expected, what happened, and the exact error:\n\n[expected]\n[actual]\n[error text]\n\nReproduce it first, then find the first point where the real state differs from the expected one. Give me at most three hypotheses with the check that would rule each out, and the smallest fix once we know which one it is.',
  },
  {
    title: 'Explain it simply',
    detail: 'The idea in plain words, then the detail that matters.',
    icon: 'sparkles',
    text: 'Explain [the idea] in plain words first, in two or three sentences, as if I know the field but not this part. Then give the detail that changes how I would use it, and one example. Name anything you are unsure about instead of guessing.',
  },
  {
    title: 'Extract a table',
    detail: 'Rows and columns from text or a file, with the blanks marked.',
    icon: 'square.grid.3x3',
    text: 'Extract this into a table with the columns [columns]. One row per [what a row is]. Keep values exactly as written, and write "missing" where a value is not there. Tell me which rows you were unsure about and why.\n\n[paste the text or attach the file]',
  },
  {
    title: 'Ask me questions first',
    detail: 'The few questions that would change the answer.',
    icon: 'info.circle',
    text: 'Before you start on [the task], ask me the few questions whose answers would change what you do. At most five, one line each, with the options you expect. If none would change it, say so and begin.',
  },
  {
    title: 'Check my writing',
    detail: 'Meaning kept, filler gone, unsupported claims flagged.',
    icon: 'pencil',
    text: 'Edit this text without changing what it means. Cut filler and stock phrases, shorten long sentences, and make the actor clear. List every fact, number and claim before and after so none disappear, and flag any claim that needs a source instead of inventing one.\n\n[paste the text]',
  },
]

/** The prompt library: a sheet of starting points that fill the composer. */
export function PromptLibrarySheet({ open, onClose, onPick }: {
  open: boolean
  onClose: () => void
  onPick: (text: string) => void
}) {
  const [query, setQuery] = useState('')
  const q = query.trim().toLowerCase()
  const shown = prompts.filter((prompt) => !q || prompt.title.toLowerCase().includes(q) || prompt.detail.toLowerCase().includes(q))
  return (
    <Sheet open={open} onClose={onClose} width={620} maxHeight="82vh" dismissOnDim>
      <VStack gap={14}>
        <ModalHeader icon="text.book.closed" title="Prompt library" subtitle="Pick one to fill the message box; edit it before sending." onClose={onClose} />
        <Field label="Search prompts" value={query} onChange={setQuery} placeholder="Search prompts" />
        <div className="prompt-library-scroll">
          {shown.length === 0 ? (
            <Text size="callout" tone="secondary">No prompt matches that.</Text>
          ) : (
            shown.map((prompt) => (
              <button key={prompt.title} type="button" className="prompt-card press" onClick={() => onPick(prompt.text)}>
                <HStack gap={10} align="start">
                  <span className="prompt-icon"><Icon name={prompt.icon} size={14} /></span>
                  <VStack gap={2} grow>
                    <Text weight="semibold">{prompt.title}</Text>
                    <Text size="caption" tone="secondary">{prompt.detail}</Text>
                  </VStack>
                </HStack>
              </button>
            ))
          )}
        </div>
      </VStack>
    </Sheet>
  )
}
