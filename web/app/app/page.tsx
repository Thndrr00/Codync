"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import {
  ArrowClockwise,
  ArrowRight,
  Eye,
  EyeSlash,
  MagnifyingGlass,
  Robot,
  Sparkle,
} from "@phosphor-icons/react";
import { botStatusLine, formatRelative } from "@/lib/host-protocol";
import type { Bot } from "@/lib/host-types";
import { useHost } from "./host-provider";
import {
  Badge,
  BotAvatar,
  button,
  EmptyState,
  iconButton,
  PageHeader,
  Skeleton,
  StatusDot,
  surface,
  useNow,
} from "./ui";

function BotCard({ bot, members, now }: { bot: Bot; members: Bot[]; now: number }) {
  return (
    <Link
      href={`/app/chat?bot=${encodeURIComponent(bot.id)}`}
      prefetch={false}
      className={`${surface} group flex flex-col gap-4 p-4 transition hover:bg-[var(--app-panel-2)]`}
    >
      <div className="flex items-start gap-3">
        <BotAvatar bot={bot} members={members} size={44} />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h2 className="truncate text-[15px] font-semibold text-[var(--app-text)]">{bot.name}</h2>
            {bot.lead ? (
              <span className="inline-flex items-center gap-1 rounded-full bg-[var(--app-panel-2)] px-1.5 py-0.5 text-[10px] font-medium text-[var(--app-warn)]">
                <Sparkle size={10} weight="fill" /> Lead
              </span>
            ) : null}
            {bot.kind === "group" ? <Badge>Group</Badge> : null}
          </div>
          <p className="mt-0.5 flex items-center gap-1.5 truncate text-xs text-[var(--app-muted)]">
            <StatusDot status={bot.status} size={7} />
            <span className="truncate">{botStatusLine(bot)}</span>
          </p>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-1.5">
          {now > 0 && bot.lastAt > 0 ? <span className="text-[11px] text-[var(--app-faint)]">{formatRelative(bot.lastAt, now)}</span> : null}
          {bot.unread > 0 ? (
            <span className="inline-flex min-w-5 items-center justify-center rounded-full bg-[var(--app-accent)] px-1.5 py-0.5 text-[10px] font-semibold text-[var(--app-accent-text)]">
              {bot.unread > 99 ? "99+" : bot.unread}
            </span>
          ) : null}
        </div>
      </div>

      {bot.description ? <p className="line-clamp-2 text-sm leading-relaxed text-[var(--app-muted)]">{bot.description}</p> : null}

      <div className="mt-auto flex items-center gap-2 text-[11px] text-[var(--app-faint)]">
        <span className="rounded-full bg-[var(--app-panel-2)] px-2 py-0.5">{bot.backend}</span>
        {bot.model ? <span className="truncate">{bot.model}</span> : null}
        {bot.kind === "group" ? <span className="truncate">{bot.members.length} members</span> : null}
        <span className="ml-auto inline-flex items-center gap-1 font-medium text-[var(--app-muted)] transition group-hover:text-[var(--app-text)]">
          Chat <ArrowRight size={12} weight="bold" />
        </span>
      </div>
    </Link>
  );
}

export default function RosterPage() {
  const { bots, hello, synced, refresh, connection } = useHost();
  const [query, setQuery] = useState("");
  const [showHidden, setShowHidden] = useState(false);
  const now = useNow();

  const hidden = bots.filter((bot) => bot.hidden || bot.archived);
  const filtered = useMemo(() => {
    const visible = bots.filter((bot) => showHidden || (!bot.hidden && !bot.archived));
    const needle = query.trim().toLowerCase();
    const matches = (bot: Bot) =>
      !needle ||
      bot.name.toLowerCase().includes(needle) ||
      bot.description.toLowerCase().includes(needle) ||
      bot.backend.toLowerCase().includes(needle) ||
      (bot.section ?? "").toLowerCase().includes(needle);
    return visible.filter(matches).sort((a, b) => {
      if (a.lead !== b.lead) return a.lead ? -1 : 1;
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      if (a.status !== b.status) {
        const rank = (status: string) => (status === "needsInput" ? 0 : status === "working" ? 1 : status === "error" ? 2 : 3);
        if (rank(a.status) !== rank(b.status)) return rank(a.status) - rank(b.status);
      }
      return b.lastAt - a.lastAt;
    });
  }, [bots, query, showHidden]);

  const pinned = filtered.filter((bot) => bot.pinned);
  const rest = filtered.filter((bot) => !bot.pinned);
  const working = bots.filter((bot) => bot.status === "working" || bot.status === "needsInput").length;

  const byId = useMemo(() => new Map(bots.map((bot) => [bot.id, bot])), [bots]);
  const membersOf = (bot: Bot) => bot.members.map((id) => byId.get(id)).filter((member): member is Bot => !!member);

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto w-full max-w-5xl px-4 py-6 sm:px-6 md:py-10">
        <PageHeader
          title="Bots"
          subtitle={
            connection === "online" ? (
              <>
                On <span className="text-[var(--app-text)]">{hello?.name || "this computer"}</span>
                {working > 0 ? ` · ${working} working` : ""}
                {bots.length > 0 ? ` · ${bots.length} total` : ""}
              </>
            ) : (
              "Your computer's bots, live."
            )
          }
          actions={
            <>
              {hidden.length > 0 ? (
                <button type="button" className={button("quiet", "sm")} onClick={() => setShowHidden((value) => !value)}>
                  {showHidden ? <EyeSlash size={14} /> : <Eye size={14} />}
                  {showHidden ? "Hide hidden" : `Show ${hidden.length} hidden`}
                </button>
              ) : null}
              <button type="button" className={iconButton()} aria-label="Refresh" title="Refresh" onClick={refresh}>
                <ArrowClockwise size={18} />
              </button>
            </>
          }
        />

        <label className="mt-6 flex h-11 items-center gap-2.5 rounded-full bg-[var(--app-panel)] px-4 transition focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-[var(--app-text)]">
          <MagnifyingGlass size={17} className="shrink-0 text-[var(--app-faint)]" />
          <span className="sr-only">Search bots</span>
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search bots"
            className="min-w-0 flex-1 bg-transparent text-sm text-[var(--app-text)] outline-none placeholder:text-[var(--app-faint)]"
          />
          {query ? (
            <button type="button" className="text-xs text-[var(--app-faint)] hover:text-[var(--app-text)]" onClick={() => setQuery("")}>
              Clear
            </button>
          ) : null}
        </label>

        {!synced ? (
          <div className="mt-6 grid gap-3 sm:grid-cols-2">
            {[0, 1, 2, 3].map((key) => (
              <Skeleton key={key} className="h-40" />
            ))}
          </div>
        ) : filtered.length === 0 ? (
          <div className="mt-6">
            <EmptyState icon={<Robot size={24} />} title={query ? "No bot matches that" : "No bots yet"}>
              {query ? (
                "Try a name, an agent or a section."
              ) : (
                <>
                  Create a bot in the Codync desktop app or on your phone. It appears here as soon as the host makes it, with its chats and
                  routines.
                </>
              )}
            </EmptyState>
          </div>
        ) : (
          <div className="mt-6 space-y-8">
            {pinned.length > 0 ? (
              <section>
                <h2 className="mb-2 px-1 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">Pinned</h2>
                <div className="grid gap-3 sm:grid-cols-2">
                  {pinned.map((bot) => (
                    <BotCard key={bot.id} bot={bot} members={membersOf(bot)} now={now} />
                  ))}
                </div>
              </section>
            ) : null}
            <section>
              {pinned.length > 0 ? (
                <h2 className="mb-2 px-1 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">All bots</h2>
              ) : null}
              <div className="grid gap-3 sm:grid-cols-2">
                {rest.map((bot) => (
                  <BotCard key={bot.id} bot={bot} members={membersOf(bot)} now={now} />
                ))}
              </div>
            </section>
          </div>
        )}

        <p className="mt-10 text-xs leading-relaxed text-[var(--app-faint)]">
          The web client is one of Codync&apos;s clients, not a copy of your computer: bots, chats and settings stay on the host and appear on every
          paired app.
        </p>
      </div>
    </div>
  );
}
