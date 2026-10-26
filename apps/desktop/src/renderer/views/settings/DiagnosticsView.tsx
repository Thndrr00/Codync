import { useEffect, useMemo, useState } from 'react'
import type { SentinelReport } from '@shared/models'
import { Button, CardForm, CardSection, Spinner, ValueRow } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { MaintenanceStore } from '../../store/maintenance-store'
import { BotBundleSection } from './BotBundleSection'
import { BotCheckSection } from './BotCheckSection'
import { TeamRunSection } from './TeamRunSection'
import { TokenBudgetSection } from './TokenBudgetSection'
import { errorText } from './parts'

/** Sentinel forensic diagnostics and health check (INIC-05). */
export function DiagnosticsView() {
  const app = useApp()
  const local = useModel(app.local)
  const running = local !== null && local.connection.kind === 'online'
  const [report, setReport] = useState<SentinelReport | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const maintenance = useModel(
    useMemo(() => new MaintenanceStore(async () => {
      if (!local) throw new Error('Start this computer’s host first.')
      return local.ready()
    }), [local]),
  )
  const bots = local ? [...local.bots.values()].filter((b) => b.kind !== 'group') : []

  const runScan = async () => {
    if (!local || !running) return
    setLoading(true)
    setError(null)
    try {
      const res = await local.sentinelHealth()
      setReport(res)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    if (running) void runScan()
  }, [running])

  return (
    <CardForm>
      <CardSection
        title="Sentinel Watchdog"
        footer="Scans SQLite integrity, orphaned browser locks, stale job assignments, and process health."
      >
        <ValueRow
          label="Status"
          detail={
            report
              ? `Last scanned ${new Date(report.timestamp).toLocaleTimeString()}`
              : running
                ? 'Ready to scan'
                : 'Host is offline'
          }
        >
          <HStack gap={8}>
            {report ? (
              <HStack gap={4}>
                <Icon
                  name={report.healthy ? 'checkmark.shield.fill' : 'exclamationmark.triangle.fill'}
                  size={13}
                  color={report.healthy ? 'var(--success, #30d158)' : 'var(--warning, #ff9f0a)'}
                />
                <Text tone={report.healthy ? 'text' : 'attention'}>
                  {report.healthy ? 'Healthy' : `${report.issues.length} issue(s)`}
                </Text>
              </HStack>
            ) : null}
            <Button kind="secondary" disabled={!running || loading} onClick={() => void runScan()}>
              {loading ? <Spinner size={12} /> : null}
              {loading ? 'Scanning…' : 'Run diagnostic'}
            </Button>
          </HStack>
        </ValueRow>
        {error ? <Text size="footnote" tone="danger">{error}</Text> : null}
      </CardSection>

      {report ? (
        <CardSection title="Diagnostic Findings">
          {report.issues.length === 0 ? (
            <HStack gap={8}>
              <Icon name="checkmark.circle.fill" size={14} color="var(--success, #30d158)" />
              <Text tone="secondary">No database, browser profile, or runtime anomalies detected.</Text>
            </HStack>
          ) : (
            report.issues.map((issue) => (
              <VStack key={issue.id} gap={4} className="settings-row">
                <HStack gap={6}>
                  <Icon
                    name={issue.severity === 'critical' ? 'xmark.octagon.fill' : issue.severity === 'warning' ? 'exclamationmark.triangle.fill' : 'info.circle'}
                    size={13}
                    color={issue.severity === 'critical' ? 'var(--danger)' : issue.severity === 'warning' ? 'var(--warning)' : 'var(--secondary)'}
                  />
                  <Text weight="semibold">{issue.category.toUpperCase()} · {issue.id}</Text>
                  {issue.autoFixable ? <Text size="caption" tone="tertiary">(Auto-fixable via codync-host sentinel --fix)</Text> : null}
                </HStack>
                <Text size="footnote" tone="secondary">{issue.message}</Text>
              </VStack>
            ))
          )}
        </CardSection>
      ) : null}
      <BotCheckSection store={maintenance} online={running} />
      <TeamRunSection bots={bots} online={running} send={(text, botId) => local?.send(text, botId)} />
      <TokenBudgetSection store={maintenance} bots={bots} online={running} />
      <BotBundleSection store={maintenance} bots={bots} online={running} />
    </CardForm>
  )
}
