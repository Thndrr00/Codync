import { Button, CardSection } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { VStack } from '../../components/Stack'
import { useStore } from '../../store/context'
import { useTr } from '../../lib/use-language'
import type { ConnectionGuide } from './market-models'

/**
 * How to connect an app (the host's `guide`): what to do at the vendor and the page to do it on.
 *
 * The steps are the vendor's own, or a reviewed copy of them; the link is only ever the vendor's page
 * (never the broker's), so somebody who expected a sign-in is never left in front of a bare form.
 */
export function ConnectionGuideCard({ guide, name }: { guide?: ConnectionGuide | null; name: string }) {
  const tr = useTr()
  const store = useStore()
  if (!guide || guide.steps.length === 0) return null
  const signingIn = guide.kind === 'signIn'
  return (
    <CardSection
      title={tr('How to connect')}
      footer={signingIn
        ? tr('The sign-in stays on {name}’s own page.').replaceAll('{name}', name)
        : tr('Keys are saved on {name} only.').replaceAll('{name}', store.hostName)}
    >
      <VStack gap={8}>
        <ol className="market-steps">
          {guide.steps.map((step) => <li key={step}>{step}</li>)}
        </ol>
        {guide.url ? (
          <div>
            <Button kind="secondary" onClick={() => window.codync.app.openExternal(guide.url!)}>
              <Icon name="safari" size={11} />
              {tr('Open {name}’s page').replaceAll('{name}', name)}
            </Button>
          </div>
        ) : null}
      </VStack>
    </CardSection>
  )
}

/** The key note a card carries before anybody adds the app. */
export function KeyNote({ text }: { text: string }) {
  return (
    <span className="market-key-note">
      <Icon name="key" size={10} />
      {text}
    </span>
  )
}
