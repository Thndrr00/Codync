import { useState } from 'react'
import { Button, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { Dialog, SheetPage } from '../../components/Overlay'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { useTr } from '../../lib/use-language'
import { errorText, needsSignIn } from './market-models'
import { AppLogo, FormSheet, ItemGrid, ItemRow, MarketSection, ScreenHeader, ServiceLogo, SkillGlyph } from './pieces'
import { SkillSheet, type SkillRef } from './SkillSheet'
import { McpToolsSheet } from './McpToolsSheet'
import { usePlugins } from './plugins'

export function InstalledView({ back, connectorId }: { back: () => void; connectorId?: string }) {
  const store = useStore()
  const tr = useTr()
  const plugins = usePlugins(store)
  const [removing, setRemoving] = useState<{ id: string; name: string; isSkill: boolean } | null>(null)
  const [signingIn, setSigningIn] = useState<string | null>(null)
  const [tools, setTools] = useState<{ id: string; name: string } | null>(null)
  const [viewingSkill, setViewingSkill] = useState<SkillRef | null>(null)
  const targeted = plugins.installedConnectors.find((connector) => connector.id === connectorId)
  const connectors = targeted ? [targeted, ...plugins.installedConnectors.filter((connector) => connector.id !== connectorId)] : plugins.installedConnectors
  const empty = plugins.installedConnectors.length === 0 && plugins.installedSkills.length === 0

  const signIn = async (id: string) => {
    setSigningIn(id)
    try {
      await plugins.signIn(id)
    } catch (e) {
      store.setError(errorText(e))
    }
    setSigningIn(null)
  }

  const remove = async (r: { id: string; isSkill: boolean }) => {
    try {
      if (r.isSkill) await plugins.removeSkill(r.id)
      else await plugins.removeConnector(r.id)
    } catch (e) {
      store.setError(errorText(e))
    }
  }

  return (
    <SheetPage>
      <ScreenHeader title="Connectors & skills" onBack={back} />
      <div className="market-installed-body">
        <div className="market-scroll">
          <div className="market-page-content">
            {connectorId && !targeted && plugins.installedConnectors.length > 0 ? <Text tone="secondary">{tr('The requested connector is no longer installed.')}</Text> : null}
            {empty ? <EmptyState icon="shippingbox" title="Nothing installed" detail="Connectors and skills you add show up here." /> : null}
            {plugins.installedConnectors.length ? (
              <MarketSection title="Connectors">
                <ItemGrid>
                  {connectors.map((c) => (
                    <ItemRow
                      key={c.id}
                      title={c.name}
                      subtitle={c.lastFailure ? `⚠ ${c.lastFailure.kind.toUpperCase()}: ${c.lastFailure.message}` : needsSignIn(c) ? 'Sign in so bots can use it' : (c.command ?? c.url ?? c.description)}
                      icon={c.kind === 'composio' ? <AppLogo url={c.logo} name={c.name} /> : <ServiceLogo name={c.name} registryName={c.registryName} />}
                      trailing={(
                        <>
                          {signingIn === c.id ? <Spinner /> : (needsSignIn(c) || c.lastFailure?.kind === 'auth') ? <Button onClick={() => void signIn(c.id)}>{c.lastFailure?.kind === 'auth' ? 'Re-authenticate' : 'Sign in'}</Button> : null}
                          <Button kind="secondary" onClick={() => setTools(c)}>Tools &amp; requests</Button>
                          <RemoveButton title={c.name} onClick={() => setRemoving({ id: c.id, name: c.name, isSkill: false })} />
                        </>
                      )}
                    />
                  ))}
                </ItemGrid>
              </MarketSection>
            ) : null}
            {plugins.installedSkills.length ? (
              <MarketSection title="Skills">
                <ItemGrid>
                  {plugins.installedSkills.map((s) => (
                    <ItemRow
                      key={s.id}
                      title={s.name}
                      subtitle={s.description}
                      icon={<SkillGlyph />}
                      onOpen={() => setViewingSkill({ id: s.id, name: s.name, description: s.description })}
                      trailing={<RemoveButton title={s.name} onClick={() => setRemoving({ id: s.id, name: s.name, isSkill: true })} />}
                    />
                  ))}
                </ItemGrid>
              </MarketSection>
            ) : null}
          </div>
        </div>
      </div>
      <Dialog
        open={removing !== null}
        title={tr('Remove {name}?').replaceAll('{name}', removing?.name ?? '')}
        message={removing ? tr(removing.isSkill ? 'Bots stop using this skill.' : 'Bots lose this connector, and the keys saved for it are deleted.') : null}
        actions={removing ? [{ title: 'Remove', destructive: true, action: () => void remove(removing) }] : []}
        onClose={() => setRemoving(null)}
      />
      <FormSheet open={viewingSkill !== null} onClose={() => setViewingSkill(null)}>
        {viewingSkill ? <SkillSheet key={viewingSkill.name} skill={viewingSkill} /> : null}
      </FormSheet>
      <FormSheet open={tools !== null} onClose={() => setTools(null)}>
        {tools && <McpToolsSheet key={`${store.contextId}:${store.computer.id}:${tools.id}`} connector={tools} />}
      </FormSheet>
    </SheetPage>
  )
}

/** A quiet "nothing here" message (replaces `ContentUnavailableView`). */
function EmptyState({ icon, title, detail }: { icon: string; title: string; detail: string }) {
  const tr = useTr()
  return (
    <VStack align="center" gap={8} className="market-empty">
      <Icon name={icon} size={28} color="var(--tertiary)" />
      <Text size="compactBody" weight="semibold" tone="text">{tr(title)}</Text>
      <Text size="compactSecondary" tone="secondary">{tr(detail)}</Text>
    </VStack>
  )
}

function RemoveButton({ title, onClick }: { title: string; onClick: () => void }) {
  return (
    <button
      title="Remove"
      aria-label={`Remove ${title}`}
      className="market-remove"
      onClick={(e) => {
        e.stopPropagation()
        onClick()
      }}
    >
      <Icon name="trash" size={13} />
    </button>
  )
}
