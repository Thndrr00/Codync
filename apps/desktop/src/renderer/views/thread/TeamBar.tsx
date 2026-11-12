import type { Bot } from '@shared/models'
import { BotAvatar } from '../../components/Avatar'
import { Icon } from '../../components/Icon'
import { HStack, Spacer } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'

/**
 * One line above a group's transcript when it works as a team: who leads, how many members
 * follow, and whether the team answers as one voice. It opens the structure editor.
 */
export function TeamBar({ bot, members, onOpen }: { bot: Bot; members: Bot[]; onOpen: () => void }) {
  const tr = useTr()
  const leadId = bot.chiefBotId ?? bot.members[0] ?? null
  const lead = members.find((m) => m.id === leadId) ?? null
  const voice = bot.team?.mode === 'smart'
    ? tr('Automatic')
    : bot.team?.voice === 'all'
      ? tr('Everyone')
      : bot.team?.voice === 'lead_and_hands'
        ? tr('Lead + named')
        : tr('Lead only')
  return (
    <button type="button" className="team-bar" onClick={onOpen} title={tr('Open the team structure')}>
      <HStack gap={8} align="center">
        <span className="team-bar-mark">
          <Icon name="person.2" size={13} />
        </span>
        {lead ? <BotAvatar bot={lead} size={20} animated={false} /> : null}
        <Text size="caption" tone="secondary">
          {lead ? `${lead.name} ${tr('leads')}` : tr('One voice')} · {members.length} {tr('agents')} · {voice}
        </Text>
        <Spacer />
        <Text size="caption" tone="tertiary">{tr('Structure')}</Text>
        <Icon name="chevron.right" size={10} />
      </HStack>
    </button>
  )
}
