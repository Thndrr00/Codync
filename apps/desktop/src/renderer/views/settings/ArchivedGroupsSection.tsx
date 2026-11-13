import { BotOrGroupAvatar } from '../../components/Avatar'
import { CardSection, IconButton, Reveal } from '../../components/Controls'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModels } from '../../lib/observable'
import { refKey } from '../../store/app-model'
import { useApp } from '../../store/context'
import { useTr } from '../../lib/use-language'

/** Groups taken out of the open roster on every computer, each with Restore (the iPhone's Archived groups). */
export function ArchivedGroupsSection() {
  const app = useApp()
  const tr = useTr()
  useModels([...app.stores.values()])
  const archived = app.archivedBots
  return (
    <Reveal show={archived.length > 0}>
      <CardSection title={tr('Archived groups')}>
        {archived.map(({ ref, bot, store }) => (
          <div key={refKey(ref)} className="settings-row">
            <BotOrGroupAvatar bot={bot} members={store.members(bot)} size={28} animated={false} />
            <VStack grow className="settings-stack">
              <Text truncate>{bot.name}</Text>
              <Text size="caption" tone="tertiary">{bot.title || store.hostName}</Text>
            </VStack>
            <IconButton title={tr('Restore')} icon="tray.and.arrow.up" onClick={() => store.setArchived(bot, false)} />
          </div>
        ))}
      </CardSection>
    </Reveal>
  )
}
