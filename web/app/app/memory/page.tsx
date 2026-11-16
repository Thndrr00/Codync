"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { ArrowClockwise, Brain, NotePencil } from "@phosphor-icons/react";
import { formatRelative, readableError } from "@/lib/host-protocol";
import type { Bot, MemoryFact, MemoryLayer, MemoryView } from "@/lib/host-types";
import { useHost } from "../host-provider";
import { button, EmptyState, ErrorNote, iconButton, PageHeader, Select, Skeleton, useNow } from "../ui";

type Section = "yours" | "bot" | "channel";

function authorOf(fact: MemoryFact, bots: Bot[], bot: Bot): string {
  if (fact.author === "user") return "You wrote it";
  if (fact.author === "dream") return "Dreaming";
  if (fact.author === "imported") return "Imported";
  if (fact.author === "bot") {
    const id = fact.authorBot;
    return id ? `${bots.find((item) => item.id === id)?.name ?? id} remembered it` : `${bot.name} remembered it`;
  }
  return "Remembered";
}

function LayerFacts({ layer, bots, bot, now }: { layer: MemoryLayer; bots: Bot[]; bot: Bot; now: number }) {
  if (!layer.entries.length) {
    return (
      <div className="flex flex-col items-center rounded-3xl bg-[var(--app-panel)] px-6 py-12 text-center">
        <Brain size={22} className="text-[var(--app-faint)]" />
        <p className="mt-3 text-sm text-[var(--app-muted)]">Nothing here yet. What your bots learn about you shows up as you talk.</p>
      </div>
    );
  }
  return (
    <div className="space-y-2">
      {layer.entries.map((fact) => (
        <article key={fact.id} className="rounded-2xl bg-[var(--app-panel)] px-4 py-3">
          <p className="text-sm leading-relaxed text-[var(--app-text)]">{fact.text}</p>
          <p className="mt-1.5 text-[11px] text-[var(--app-faint)]">
            {authorOf(fact, bots, bot)}
            {now > 0 && fact.createdAt > 0 ? ` · ${formatRelative(fact.createdAt, now)}` : ""}
          </p>
        </article>
      ))}
    </div>
  );
}

export default function MemoryPage() {
  const { bots, client, synced } = useHost();
  const [botId, setBotId] = useState("");
  const [section, setSection] = useState<Section>("bot");
  const [view, setView] = useState<MemoryView | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const now = useNow();

  useEffect(() => {
    if (!botId && bots.length) setBotId((bots.find((bot) => bot.kind !== "group") ?? bots[0]).id);
  }, [bots, botId]);

  const bot = bots.find((item) => item.id === botId);

  useEffect(() => {
    if (!botId) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    void client
      .memory(botId)
      .then((answer) => {
        if (!cancelled) setView(answer);
      })
      .catch((failure) => {
        if (!cancelled) setError(readableError(failure));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [botId, client, bot?.memoryRev, tick]);

  useEffect(() => {
    setSection("bot");
  }, [botId]);

  useEffect(() => {
    if (section === "channel" && !view?.channel) setSection("bot");
  }, [section, view?.channel]);

  const reload = useCallback(() => setTick((value) => value + 1), []);
  const sections = useMemo(() => {
    const list: { id: Section; label: string }[] = [
      { id: "yours", label: "About you" },
      { id: "bot", label: "This bot" },
    ];
    if (view?.channel) list.push({ id: "channel", label: "This room" });
    return list;
  }, [view]);
  const layer = view ? (section === "yours" ? view.yours : section === "bot" ? view.bot : view.channel ?? view.bot) : null;
  const percent = layer && layer.cap > 0 ? Math.min(100, Math.round((layer.used / layer.cap) * 100)) : 0;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto w-full max-w-3xl px-4 py-6 sm:px-6 md:py-10">
        <PageHeader
          title="Memory"
          subtitle="What your bots remember about you, kept as plain Markdown on your computer."
          actions={
            <>
              <Select value={botId} onChange={setBotId} label="Bot">
                {bots.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </Select>
              <button type="button" className={iconButton()} aria-label="Refresh" title="Refresh" onClick={reload}>
                <ArrowClockwise size={18} />
              </button>
            </>
          }
        />

        <div className="mt-5 flex items-start gap-2.5 rounded-2xl bg-[var(--app-panel)] px-4 py-3">
          <NotePencil size={16} className="mt-0.5 shrink-0 text-[var(--app-muted)]" />
          <p className="text-xs leading-relaxed text-[var(--app-muted)]">
            Memory is read-only here. Change it by telling the bot, or edit the Markdown files on your computer.
          </p>
        </div>

        {error ? (
          <div className="mt-3">
            <ErrorNote>{error}</ErrorNote>
          </div>
        ) : null}

        {!botId && !bots.length && synced ? (
          <div className="mt-6">
            <EmptyState icon={<Brain size={24} />} title="No bots to remember yet">
              Create a bot on your computer first; its memory appears here.
            </EmptyState>
          </div>
        ) : loading && !view ? (
          <div className="mt-6 space-y-2">
            <Skeleton className="h-20" />
            <Skeleton className="h-16" />
            <Skeleton className="h-16" />
          </div>
        ) : view && bot ? (
          <>
            <div className="mt-5 flex flex-wrap items-center gap-2">
              {sections.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  aria-pressed={section === item.id}
                  onClick={() => setSection(item.id)}
                  className={button(section === item.id ? "primary" : "quiet", "sm")}
                >
                  {item.label}
                </button>
              ))}
            </div>

            {layer ? (
              <div className="mt-4">
                <div className="flex items-center justify-between gap-3 text-xs text-[var(--app-muted)]">
                  <p className="leading-relaxed">
                    {section === "yours"
                      ? "Your name, time zone, people and lasting preferences, shared by your bots."
                      : section === "channel"
                        ? "What this room keeps, beside its members' own memory."
                        : `What ${bot.name} learned about you.`}
                  </p>
                  <span className="shrink-0 tabular-nums">
                    {layer.used} / {layer.cap}
                  </span>
                </div>
                <div className="mt-2 h-1 overflow-hidden rounded-full bg-[var(--app-panel-2)]">
                  <div className="h-full rounded-full bg-[var(--app-text)] transition-[width]" style={{ width: `${percent}%` }} />
                </div>
                <div className="mt-4">
                  <LayerFacts layer={layer} bots={bots} bot={bot} now={now} />
                </div>
              </div>
            ) : null}

            {section === "bot" && view.projectMemory ? (
              <section className="mt-6">
                <h2 className="mb-2 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">The agent&apos;s own notes</h2>
                <div className="rounded-2xl bg-[var(--app-panel)] p-4">
                  <p className="text-xs leading-relaxed text-[var(--app-muted)]">
                    Read-only. The agent writes this outside Codync; it is shown so nothing it keeps is out of sight.
                  </p>
                  {view.projectMemory.text ? (
                    <pre className="mt-3 max-h-60 overflow-auto rounded-xl bg-[var(--app-panel-2)] p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap text-[var(--app-text)]">
                      {view.projectMemory.text}
                    </pre>
                  ) : (
                    <p className="mt-2 text-xs text-[var(--app-faint)]">Nothing saved yet.</p>
                  )}
                </div>
              </section>
            ) : null}
          </>
        ) : (
          <div className="mt-6 space-y-2">
            <Skeleton className="h-20" />
            <Skeleton className="h-16" />
          </div>
        )}

        <p className="mt-8 text-xs leading-relaxed text-[var(--app-faint)]">
          Every app reads the same files. Memory changes made anywhere move the bot&apos;s revision, so this page refreshes on its own.
        </p>
      </div>
    </div>
  );
}
