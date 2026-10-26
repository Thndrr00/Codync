import { useEffect, useState } from 'react'
import type { AppLink, BotApp } from '@shared/models'
import { Button, SegmentedChoice } from './Controls'
import { Icon } from './Icon'
import { SlideShell, useLatchedWhileClosing } from './SlideShell'
import { HStack, Spacer, VStack } from './Stack'
import { Text } from './Text'

/** Ideas 1–7, 47, 48: Sandboxed mini-app viewer drawer with capability share links and error banner. */
export function BotAppPanel({
  app,
  onClose,
  onCreateShareLink,
  onRevokeShareLink,
}: {
  app: BotApp | null
  onClose: () => void
  onCreateShareLink?: (slug: string, label: string, mode: 'view' | 'edit') => Promise<{ link: AppLink; url: string }>
  onRevokeShareLink?: (slug: string, token: string) => Promise<void>
}) {
  const latched = useLatchedWhileClosing(app)
  const [reloadKey, setReloadKey] = useState(0)
  const [shareOpen, setShareOpen] = useState(false)
  const [shareLabel, setShareLabel] = useState('Team link')
  const [shareMode, setShareMode] = useState<'view' | 'edit'>('view')
  const [copiedUrl, setCopiedUrl] = useState<string | null>(null)
  const [shareError, setShareError] = useState<string | null>(null)
  const [creatingLink, setCreatingLink] = useState(false)
  const [runtimeError, setRuntimeError] = useState<string | null>(null)

  useEffect(() => {
    setRuntimeError(null)
    setCopiedUrl(null)
    setShareError(null)
    const onMessage = (ev: MessageEvent) => {
      if (
        ev.data &&
        typeof ev.data === 'object' &&
        ev.data.type === 'bdots-app-error' &&
        ev.data.slug === latched?.slug
      ) {
        setRuntimeError(String(ev.data.message || 'Runtime error'))
      }
    }
    window.addEventListener('message', onMessage)
    return () => window.removeEventListener('message', onMessage)
  }, [latched?.slug, latched?.version])

  return (
    <SlideShell
      open={app !== null}
      onClose={onClose}
      ariaLabel={latched?.title ?? 'Mini-app'}
      width="min(680px, 94vw)"
    >
      {latched ? (
        <VStack style={{ height: '100%' }}>
          <HStack
            align="center"
            gap={10}
            style={{
              padding: '12px 16px',
              borderBottom: '1px solid var(--border)',
            }}
          >
            <VStack gap={2}>
              <HStack align="center" gap={6}>
                <Text size="body" weight="semibold">
                  {latched.title}
                </Text>
                <span className="t-tab-badge">v{latched.version}</span>
              </HStack>
              {latched.description ? (
                <Text size="caption" tone="secondary">
                  {latched.description}
                </Text>
              ) : null}
            </VStack>
            <Spacer />
            <Button kind="secondary" onClick={() => setShareOpen(!shareOpen)}>
              Share
            </Button>
            <Button kind="secondary" onClick={() => setReloadKey((k) => k + 1)} title="Reload app">
              <Icon name="arrow.clockwise" size={12} />
            </Button>
            <Button kind="secondary" onClick={onClose} title="Close">
              <Icon name="xmark" size={12} />
            </Button>
          </HStack>

          {runtimeError ? (
            <HStack
              align="center"
              gap={8}
              style={{
                padding: '8px 16px',
                background: 'color-mix(in srgb, var(--danger) 14%, var(--surface))',
                color: 'var(--danger)',
              }}
            >
              <Icon name="exclamationmark.triangle.fill" size={13} color="var(--danger)" />
              <Text size="caption" tone="danger">
                Auto-repairing runtime error: {runtimeError}
              </Text>
            </HStack>
          ) : null}

          <div className={`t-resize ${shareOpen ? 'is-open' : ''}`}>
            <div>
              <VStack
                gap={10}
                style={{
                  padding: '12px 16px',
                  background: 'var(--group)',
                  borderBottom: '1px solid var(--border)',
                }}
              >
                <HStack align="center" gap={8}>
                  <input
                    value={shareLabel}
                    onChange={(e) => setShareLabel(e.target.value)}
                    placeholder="Link label (e.g. Friends)"
                    style={{
                      flex: 1,
                      padding: '6px 10px',
                      borderRadius: 'var(--radius-s)',
                      background: 'var(--field)',
                    }}
                  />
                  <SegmentedChoice
                    selection={shareMode}
                    options={[
                      { id: 'view', label: 'View only' },
                      { id: 'edit', label: 'Can edit' },
                    ]}
                    onChange={setShareMode}
                  />
                  <Button
                    kind="primary"
                    disabled={!onCreateShareLink || creatingLink}
                    onClick={async () => {
                      if (!onCreateShareLink) return
                      setCreatingLink(true)
                      setCopiedUrl(null)
                      setShareError(null)
                      try {
                        const res = await onCreateShareLink(latched.slug, shareLabel, shareMode)
                        window.codync.app.copy(res.url)
                        setCopiedUrl(res.url)
                      } catch (error) {
                        setShareError(error instanceof Error ? error.message : 'Could not create the share link.')
                      } finally {
                        setCreatingLink(false)
                      }
                    }}
                  >
                    {creatingLink ? 'Creating…' : 'Create & Copy Link'}
                  </Button>
                </HStack>
                {shareError ? <Text size="caption" tone="danger">{shareError}</Text> : null}
                {copiedUrl ? (
                  <Text size="caption" tone="secondary">
                    Copied capability URL: {copiedUrl}
                  </Text>
                ) : null}
                {latched.links && latched.links.length > 0 ? (
                  <VStack gap={4}>
                    {latched.links.map((l) => (
                      <HStack key={l.token} align="center" gap={8}>
                        <Text size="caption">{l.label}</Text>
                        <span className="t-tab-badge">{l.mode}</span>
                        <Text size="caption" tone="secondary">
                          {l.opens} opens
                        </Text>
                        <Spacer />
                        {onRevokeShareLink ? (
                          <button
                            type="button"
                            onClick={() => onRevokeShareLink(latched.slug, l.token)}
                            style={{ color: 'var(--danger)', fontSize: 11 }}
                          >
                            Revoke
                          </button>
                        ) : null}
                      </HStack>
                    ))}
                  </VStack>
                ) : null}
              </VStack>
            </div>
          </div>

          <iframe
            key={`${latched.slug}-${latched.version}-${reloadKey}`}
            src={latched.url}
            title={latched.title}
            sandbox="allow-scripts allow-forms allow-popups allow-modals allow-downloads"
            style={{
              flex: 1,
              width: '100%',
              border: 'none',
              background: 'var(--background)',
            }}
          />
        </VStack>
      ) : null}
    </SlideShell>
  )
}
