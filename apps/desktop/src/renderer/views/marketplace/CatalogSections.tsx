import { useState } from 'react'
import { fromProfile, teamProfiles, type BotCatalog, type BotProfile, type TeamProfile } from '@shared/catalog'
import { CharacterAvatar } from '../../components/Avatar'
import { CardForm, CardSection, IconButton, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, SheetPage } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { useTr } from '../../lib/use-language'
import { errorText, fillDefaults, newDraft } from '../bots/drafts'
import { useCatalog } from '../bots/use-catalog'
import { FormSheet, ItemGrid, MarketRow, MarketSection } from './pieces'

/** A catalog entry the sheet shows. */
type Item = { kind: 'bot'; profile: BotProfile } | { kind: 'team'; team: TeamProfile }
const itemId = (item: Item) => (item.kind === 'bot' ? item.profile.id : item.team.id)

/**
 * The Extensions page's ready-made bots and coordinator teams (host `botCatalog`). Adding a bot creates it on
 * the computer's defaults; adding a team creates its coordinator, its teammates and their group (`installTeam`).
 */
export function CatalogSections({ query, onError }: { query: string; onError: (message: string) => void }) {
  const store = useStore()
  const { catalog } = useCatalog(store)
  const [added, setAdded] = useState<string[]>([])
  const [busy, setBusy] = useState<string | null>(null)
  const [open, setOpen] = useState<Item | null>(null)
  if (!catalog) return null

  const q = query.toLowerCase()
  const matches = (name: string, summary: string) => !q || name.toLowerCase().includes(q) || summary.toLowerCase().includes(q)
  const bots = catalog.bots.filter((b) => matches(b.name, b.summary))
  const teams = catalog.teams.filter((t) => matches(t.name, t.summary))

  const add = async (item: Item) => {
    const id = itemId(item)
    setBusy(id)
    try {
      if (item.kind === 'bot') await store.save(fillDefaults(store, fromProfile(newDraft(), item.profile)))
      else await store.installTeam(item.team.id)
      setAdded((list) => [...list, id])
    } catch (e) {
      onError(errorText(e))
    }
    setBusy(null)
  }

  const row = (item: Item, title: string, subtitle: string, icon: React.ReactNode) => (
    <MarketRow
      key={itemId(item)}
      title={title}
      subtitle={subtitle}
      added={added.includes(itemId(item))}
      busy={busy === itemId(item)}
      icon={icon}
      onAdd={() => void add(item)}
      onOpen={() => setOpen(item)}
    />
  )

  return (
    <>
      {bots.length ? (
        <MarketSection title="Bots">
          <ItemGrid>
            {bots.map((p) => row({ kind: 'bot', profile: p }, p.name, p.summary, <ProfileFace profile={p} size={46} />))}
          </ItemGrid>
        </MarketSection>
      ) : null}
      {teams.length ? (
        <MarketSection title="Coordinator teams">
          <ItemGrid>
            {teams.map((t) => row({ kind: 'team', team: t }, t.name, t.summary, <TeamFace catalog={catalog} team={t} />))}
          </ItemGrid>
        </MarketSection>
      ) : null}
      <FormSheet open={open !== null} onClose={() => setOpen(null)}>
        {open ? (
          <CatalogSheet
            key={itemId(open)}
            item={open}
            catalog={catalog}
            added={added.includes(itemId(open))}
            busy={busy === itemId(open)}
            onAdd={() => void add(open)}
          />
        ) : null}
      </FormSheet>
    </>
  )
}

function ProfileFace({ profile, size }: { profile: BotProfile; size: number }) {
  return <CharacterAvatar shape={profile.avatarShape} color={profile.avatarColor} size={size} />
}

/** The coordinator in front, its first teammate peeking behind. */
function TeamFace({ catalog, team }: { catalog: BotCatalog; team: TeamProfile }) {
  const [lead, first] = teamProfiles(catalog, team)
  return (
    <HStack as="span" align="flex-end" className="catalog-team-face">
      {first ? <ProfileFace profile={first} size={28} /> : null}
      <ProfileFace profile={lead!} size={36} />
    </HStack>
  )
}

/** A ready-made bot's or team's details: its instructions, or who leads and who is on the team, with Add. */
function CatalogSheet({ item, catalog, added, busy, onAdd }: { item: Item; catalog: BotCatalog; added: boolean; busy: boolean; onAdd: () => void }) {
  const tr = useTr()
  const title = item.kind === 'bot' ? item.profile.name : item.team.name
  const summary = item.kind === 'bot' ? item.profile.summary : item.team.summary
  const trailing = added ? (
    <span title={tr('Added')} aria-label={tr('Added')} className="skill-added">
      <Icon name="checkmark" size={13} weight="semibold" />
    </span>
  ) : busy ? <Spinner /> : <IconButton title="Add" icon="plus" onClick={onAdd} />

  return (
    <SheetPage>
      <ModalHeader icon={item.kind === 'bot' ? <ProfileFace profile={item.profile} size={28} /> : 'person.2'} title={title} trailing={trailing} />
      <CardForm>
        <CardSection>
          <Text size="subheadline" tone="secondary">{summary}</Text>
        </CardSection>
        {item.kind === 'bot' ? (
          <CardSection title="Instructions" footer="You can change them in the bot's settings after adding it.">
            <Text tone="text" className="selectable">{item.profile.instructions}</Text>
          </CardSection>
        ) : (
          <CardSection title="Team" footer="The coordinator leads the group chat and hands each request to the right teammate. Each bot can be changed in its settings.">
            {teamProfiles(catalog, item.team).map((p, index) => (
              <HStack key={p.id} gap={12} align="flex-start">
                <ProfileFace profile={p} size={32} />
                <VStack gap={2} shrink>
                  <Text weight="semibold" tone="text">{index === 0 ? tr('{name} · Coordinator').replaceAll('{name}', p.name) : p.name}</Text>
                  <Text size="caption" tone="secondary">{p.summary}</Text>
                </VStack>
              </HStack>
            ))}
          </CardSection>
        )}
      </CardForm>
    </SheetPage>
  )
}
