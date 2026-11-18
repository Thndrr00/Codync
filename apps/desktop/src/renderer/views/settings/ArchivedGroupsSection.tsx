import { GroupAvatar } from '../../components/Avatar'
import { CardSection, IconButton, Reveal } from '../../components/Controls'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModels } from '../../lib/observable'
import { refKey } from '../../store/app-model'
import { useApp } from '../../store/context'

/** Groups taken out of the open roster on every computer, each with Restore (the iPhone's Archived groups). */
export function ArchivedGroupsSection() {
  const app = useApp()
  useModels([...app.stores.values()])
  const archived = app.archivedBots
  return (
    <Reveal show={archived.length > 0}>
      <CardSection title="Archived groups">
        {archived.map(({ ref, bot, store }) => (
          <div key={refKey(ref)} className="settings-row">
            <GroupAvatar members={store.members(bot)} size={28} animated={false} />
            <VStack grow className="settings-stack">
              <Text truncate>{bot.name}</Text>
              <Text size="caption" tone="tertiary">{bot.title || store.hostName}</Text>
            </VStack>
            <IconButton title="Restore" icon="tray.and.arrow.up" onClick={() => store.setArchived(bot, false)} />
          </div>
        ))}
      </CardSection>
    </Reveal>
  )
}
