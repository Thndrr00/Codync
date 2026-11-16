"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { ArrowClockwise, ArrowRight, ClockCountdown, Play } from "@phosphor-icons/react";
import { formatRelative, readableError, runStatusLabel } from "@/lib/host-protocol";
import type { Bot, Routine, RoutineRun, RoutinesAnswer } from "@/lib/host-types";
import { useHost } from "../host-provider";
import { Badge, BotAvatar, button, EmptyState, ErrorNote, iconButton, PageHeader, Skeleton, useNow } from "../ui";

function triggerText(routine: Routine): string {
  const described = (routine.triggerDescriptions ?? []).filter(Boolean);
  if (described.length) return described.join(" · ");
  return routine.triggers
    .map((trigger) =>
      trigger.type === "interval" && trigger.seconds
        ? `Every ${Math.round(trigger.seconds / 60)} minutes`
        : trigger.type === "cron" && trigger.expression
          ? trigger.expression
          : trigger.type,
    )
    .join(" · ");
}

function runTone(status: string | undefined): "quiet" | "good" | "warn" | "bad" {
  if (status === "succeeded") return "good";
  if (status === "failed" || status === "interrupted") return "bad";
  if (status === "running" || status === "starting" || status === "pending" || status === "recovering") return "warn";
  return "quiet";
}

function Switch({ checked, onChange, label }: { checked: boolean; onChange: (next: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-10 shrink-0 rounded-full transition ${checked ? "bg-[var(--app-good)]" : "bg-[var(--app-panel-2)]"}`}
    >
      <span
        aria-hidden
        className={`absolute top-1 size-4 rounded-full bg-[var(--app-bg)] transition-all ${checked ? "left-5" : "left-1"}`}
      />
    </button>
  );
}

function RoutineRow({
  routine,
  lastRun,
  bot,
  busy,
  now,
  onToggle,
  onRun,
}: {
  routine: Routine;
  lastRun?: RoutineRun;
  bot: Bot;
  busy: boolean;
  now: number;
  onToggle: (next: boolean) => void;
  onRun: () => void;
}) {
  return (
    <article className="rounded-3xl bg-[var(--app-panel)] p-4">
      <div className="flex flex-wrap items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-sm font-semibold text-[var(--app-text)]">{routine.name}</h3>
            {lastRun ? <Badge tone={runTone(lastRun.status)}>{runStatusLabel(lastRun.status)}</Badge> : <Badge>Never run</Badge>}
            {!routine.enabled ? <Badge>Paused</Badge> : null}
          </div>
          <p className="mt-1 text-xs leading-relaxed text-[var(--app-muted)]">{triggerText(routine)}</p>
          {routine.enabled && routine.nextRunAt && now > 0 ? (
            <p className="mt-1 text-xs text-[var(--app-faint)]">Next {formatRelative(routine.nextRunAt, now)}</p>
          ) : null}
          {lastRun?.detail ? <p className="mt-2 line-clamp-2 text-xs leading-relaxed text-[var(--app-faint)]">{lastRun.detail}</p> : null}
          {routine.lastError ? <p className="mt-2 text-xs leading-relaxed text-[var(--app-bad)]">{routine.lastError}</p> : null}
          {lastRun?.finishedAt && now > 0 ? (
            <p className="mt-1 text-[11px] text-[var(--app-faint)]">Last run {formatRelative(lastRun.finishedAt, now)}</p>
          ) : null}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Switch checked={routine.enabled} label={`${routine.enabled ? "Pause" : "Resume"} ${routine.name}`} onChange={onToggle} />
          <button type="button" className={button("quiet", "sm")} disabled={busy} onClick={onRun}>
            <Play size={12} weight="fill" /> {busy ? "Running…" : "Run now"}
          </button>
        </div>
      </div>
      <div className="mt-3 flex items-center gap-2 border-t border-[var(--app-line)] pt-3 text-[11px] text-[var(--app-faint)]">
        <BotAvatar bot={bot} size={18} />
        <span>{bot.name}</span>
        <Link
          href={`/app/chat?bot=${encodeURIComponent(bot.id)}`}
          prefetch={false}
          className="ml-auto inline-flex items-center gap-1 text-[var(--app-muted)] transition hover:text-[var(--app-text)]"
        >
          Open chat <ArrowRight size={11} weight="bold" />
        </Link>
      </div>
    </article>
  );
}

export default function RoutinesPage() {
  const { bots, client, subscribe } = useHost();
  const [answers, setAnswers] = useState<Record<string, RoutinesAnswer>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const now = useNow();
  const botsRef = useRef(bots);
  useEffect(() => {
    botsRef.current = bots;
  }, [bots]);

  const ids = useMemo(
    () => bots.filter((bot) => bot.kind !== "group" && !bot.hidden && !bot.archived).map((bot) => bot.id),
    [bots],
  );
  const idKey = ids.join("|");

  useEffect(() => {
    let cancelled = false;
    const targets = botsRef.current.filter((bot) => bot.kind !== "group" && !bot.hidden && !bot.archived);
    if (!targets.length) {
      setLoading(false);
      setAnswers({});
      return;
    }
    setLoading(true);
    void Promise.allSettled(targets.map(async (bot) => [bot.id, await client.routines(bot.id)] as const)).then((results) => {
      if (cancelled) return;
      const next: Record<string, RoutinesAnswer> = {};
      let failure: string | null = null;
      for (const result of results) {
        if (result.status === "fulfilled") next[result.value[0]] = result.value[1];
        else failure = readableError(result.reason);
      }
      setAnswers(next);
      setError(failure);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [client, idKey, tick]);

  useEffect(() => {
    return subscribe((event) => {
      if (event.type === "routineFlow") setTick((value) => value + 1);
    });
  }, [subscribe]);

  const refresh = useCallback(() => setTick((value) => value + 1), []);

  const act = useCallback(
    async (key: string, action: () => Promise<unknown>, message: string) => {
      setBusy(key);
      setError(null);
      setNotice(null);
      try {
        await action();
        setNotice(message);
        setTimeout(() => setTick((value) => value + 1), 900);
      } catch (failure) {
        setError(readableError(failure));
      } finally {
        setBusy(null);
      }
    },
    [],
  );

  const rows = ids.flatMap((botId) =>
    (answers[botId]?.routines ?? []).map((routine) => ({
      bot: bots.find((item) => item.id === botId),
      routine,
      lastRun: answers[botId]?.runs.find((run) => run.routineId === routine.id),
    })),
  );
  const active = rows.filter((row) => row.routine.enabled).length;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto w-full max-w-4xl px-4 py-6 sm:px-6 md:py-10">
        <PageHeader
          title="Routines"
          subtitle={rows.length ? `${rows.length} total · ${active} enabled` : "Schedules and webhooks that make your bots work on their own."}
          actions={
            <>
              <Link href="/docs/routines" prefetch={false} className={button("quiet", "sm")}>
                How routines work
              </Link>
              <button type="button" className={iconButton()} aria-label="Refresh" title="Refresh" onClick={refresh}>
                <ArrowClockwise size={18} />
              </button>
            </>
          }
        />

        {error && rows.length > 0 ? (
          <div className="mt-4">
            <ErrorNote>{error}</ErrorNote>
          </div>
        ) : null}
        {notice ? <p className="mt-4 text-xs text-[var(--app-muted)]">{notice}</p> : null}

        {loading && !rows.length ? (
          <div className="mt-6 space-y-3">
            {[0, 1, 2].map((key) => (
              <Skeleton key={key} className="h-28" />
            ))}
          </div>
        ) : rows.length === 0 ? (
          <div className="mt-6">
            <EmptyState icon={<ClockCountdown size={24} />} title="No routines yet">
              Ask a bot to set one up, or create it in the Codync desktop app. The web client can run and pause them; editing stays in the desktop and
              phone apps for now.
            </EmptyState>
          </div>
        ) : (
          <div className="mt-6 space-y-3">
            {rows.map(({ bot, routine, lastRun }) =>
              bot ? (
                <RoutineRow
                  key={`${bot.id}:${routine.id}`}
                  routine={routine}
                  lastRun={lastRun}
                  bot={bot}
                  now={now}
                  busy={busy === `${bot.id}:${routine.id}`}
                  onToggle={(enabled) =>
                    void act(
                      `${bot.id}:${routine.id}`,
                      () => client.setRoutineEnabled(bot.id, routine.id, enabled),
                      `${routine.name} ${enabled ? "resumed" : "paused"}.`,
                    )
                  }
                  onRun={() =>
                    void act(`${bot.id}:${routine.id}`, () => client.runRoutine(bot.id, routine.id), `${routine.name} queued.`)
                  }
                />
              ) : null,
            )}
          </div>
        )}

        {error && rows.length === 0 ? (
          <div className="mt-4">
            <ErrorNote>{error}</ErrorNote>
          </div>
        ) : null}

        <p className="mt-8 text-xs leading-relaxed text-[var(--app-faint)]">
          Routine runs post into the bot&apos;s chat like any other turn. A full editor, the routine&apos;s flow canvas and webhook setup stay in the
          desktop and phone apps.
        </p>
      </div>
    </div>
  );
}
