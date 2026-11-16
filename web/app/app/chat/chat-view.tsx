"use client";

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { ArrowLeft, ChatCircleDots, PaperPlaneTilt, Stop } from "@phosphor-icons/react";
import { isChatEntry, mergeEntries, readableError } from "@/lib/host-protocol";
import type { Entry } from "@/lib/host-types";
import { useHost } from "../host-provider";
import { BotAvatar, button, EmptyState, ErrorNote, iconButton, Skeleton, StatusDot } from "../ui";
import BotChooser from "./bot-chooser";
import { ChatMessage } from "./message";


export default function ChatView() {
  const params = useSearchParams();
  const botId = params.get("bot") ?? "";
  const { bots, client, synced, subscribe, connection } = useHost();
  const bot = bots.find((item) => item.id === botId);

  const [entries, setEntries] = useState<Entry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const [answering, setAnswering] = useState(false);
  const [confirmNew, setConfirmNew] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loadingOlder, setLoadingOlder] = useState(false);

  const scroller = useRef<HTMLDivElement>(null);
  const stick = useRef(true);
  const keepHeight = useRef<number | null>(null);
  const composer = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!botId) return;
    let cancelled = false;
    setEntries([]);
    setLoading(true);
    setError(null);
    setHasMore(false);
    void client
      .history(botId)
      .then((answer) => {
        if (cancelled) return;
        setEntries((current) => mergeEntries(current, answer.entries.filter((entry) => !entry.threadId)));
        setHasMore(answer.entries.length >= 100);
        setLoading(false);
        stick.current = true;
      })
      .catch((failure) => {
        if (cancelled) return;
        setError(readableError(failure));
        setLoading(false);
      });
    void client.markRead(botId).catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [botId, client]);

  useEffect(() => {
    if (!botId) return;
    return subscribe((event) => {
      if (event.type === "entry" && event.entry.botId === botId && !event.entry.threadId) {
        setEntries((current) => mergeEntries(current, [event.entry]));
      } else if (event.type === "entryDeleted") {
        setEntries((current) => current.filter((entry) => entry.id !== event.id));
      } else if (event.type === "historyCleared" && event.botId === botId) {
        setEntries([]);
      }
    });
  }, [botId, subscribe]);

  useEffect(() => {
    if (!botId || !entries.length) return;
    const timer = setTimeout(() => {
      void client.markRead(botId).catch(() => {});
    }, 900);
    return () => clearTimeout(timer);
  }, [botId, client, entries.length]);

  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element) return;
    if (keepHeight.current !== null) {
      element.scrollTop = element.scrollHeight - keepHeight.current;
      keepHeight.current = null;
    } else if (stick.current) {
      element.scrollTop = element.scrollHeight;
    }
  }, [entries]);

  useEffect(() => {
    const element = scroller.current;
    if (element && stick.current) element.scrollTop = element.scrollHeight;
  }, [bot?.status]);

  const shown = useMemo(() => entries.filter(isChatEntry), [entries]);
  const working = bot?.status === "working" || bot?.status === "needsInput";

  const loadOlder = useCallback(async () => {
    const oldest = entries[0]?.seq;
    if (!botId || oldest === undefined) return;
    const element = scroller.current;
    keepHeight.current = element ? element.scrollHeight - element.scrollTop : null;
    setLoadingOlder(true);
    try {
      const answer = await client.history(botId, oldest);
      setEntries((current) => mergeEntries(current, answer.entries.filter((entry) => !entry.threadId)));
      setHasMore(answer.entries.length >= 100);
    } catch (failure) {
      setError(readableError(failure));
    } finally {
      setLoadingOlder(false);
    }
  }, [botId, client, entries]);

  const send = useCallback(async () => {
    const value = text.trim();
    if (!botId || !value || sending) return;
    setSending(true);
    setError(null);
    setNotice(null);
    const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    try {
      const answer = await client.send(botId, value, nonce);
      setEntries((current) => mergeEntries(current, [answer.entry]));
      setText("");
      stick.current = true;
    } catch (failure) {
      setError(readableError(failure));
    } finally {
      setSending(false);
    }
  }, [botId, client, sending, text]);

  const respond = useCallback(
    async (entryId: string, optionId: string) => {
      setAnswering(true);
      setError(null);
      try {
        await client.respondPermission(entryId, optionId);
      } catch (failure) {
        setError(readableError(failure));
      } finally {
        setAnswering(false);
      }
    },
    [client],
  );

  const startOver = useCallback(async () => {
    if (!botId) return;
    setConfirmNew(false);
    setError(null);
    try {
      await client.newSession(botId);
      setEntries([]);
      setNotice("Started a new conversation. The old one stays on your computer.");
    } catch (failure) {
      setError(readableError(failure));
    }
  }, [botId, client]);

  if (!botId) return <BotChooser bots={bots} />;

  if (!synced && !bot) {
    return (
      <div className="h-full overflow-hidden">
        <div className="mx-auto max-w-3xl space-y-3 px-4 py-6">
          <Skeleton className="h-12" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
        </div>
      </div>
    );
  }

  if (!bot) {
    return (
      <div className="h-full overflow-y-auto px-4 py-10">
        <EmptyState icon={<ChatCircleDots size={24} />} title="That bot isn't here">
          It may have been deleted, or it belongs to another computer.{" "}
          <Link href="/app" prefetch={false} className="text-[var(--app-text)] underline underline-offset-4">
            Back to bots
          </Link>
          .
        </EmptyState>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-center gap-3 bg-[var(--app-bg)] px-3 py-2.5 sm:px-5">
        <Link href="/app" prefetch={false} className={iconButton("md:hidden")} aria-label="Back to bots">
          <ArrowLeft size={19} />
        </Link>
        <BotAvatar bot={bot} size={34} />
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-sm font-semibold text-[var(--app-text)]">{bot.name}</h1>
          <p className="flex items-center gap-1.5 truncate text-xs text-[var(--app-muted)]">
            <StatusDot status={bot.status} size={6} />
            {bot.status === "working" ? bot.activity || "Working" : bot.status === "needsInput" ? "Waiting for you" : connection === "online" ? bot.backend : "Offline"}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {working ? (
            <button
              type="button"
              className={button("quiet", "sm")}
              onClick={() => void client.stop(bot.id).catch((failure) => setError(readableError(failure)))}
            >
              <Stop size={13} weight="fill" /> Stop
            </button>
          ) : null}
          {confirmNew ? (
            <span className="flex items-center gap-1 rounded-full bg-[var(--app-panel)] p-1 pl-3 text-xs text-[var(--app-muted)]">
              Start over?
              <button type="button" className={button("primary", "sm")} onClick={() => void startOver()}>
                Yes
              </button>
              <button type="button" className={button("quiet", "sm")} onClick={() => setConfirmNew(false)}>
                No
              </button>
            </span>
          ) : (
            <button type="button" className={button("quiet", "sm")} onClick={() => setConfirmNew(true)}>
              New session
            </button>
          )}
        </div>
      </header>

      <div
        ref={scroller}
        className="min-h-0 flex-1 overflow-y-auto"
        onScroll={(event) => {
          const element = event.currentTarget;
          stick.current = element.scrollHeight - element.scrollTop - element.clientHeight < 80;
        }}
      >
        <div className="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-3 px-3 py-4 sm:px-5">
          {hasMore ? (
            <button type="button" className="mx-auto text-xs text-[var(--app-faint)] hover:text-[var(--app-text)]" disabled={loadingOlder} onClick={() => void loadOlder()}>
              {loadingOlder ? "Loading…" : "Load earlier messages"}
            </button>
          ) : null}

          {loading ? (
            <div className="space-y-3">
              <Skeleton className="ml-auto h-16 w-2/3" />
              <Skeleton className="h-24 w-4/5" />
              <Skeleton className="ml-auto h-12 w-1/2" />
            </div>
          ) : shown.length === 0 ? (
            <div className="flex flex-1 flex-col items-center justify-center py-16 text-center">
              <BotAvatar bot={bot} size={64} />
              <h2 className="mt-5 text-lg font-semibold text-[var(--app-text)]">Talk to {bot.name}</h2>
              <p className="mt-1 max-w-sm text-sm leading-relaxed text-[var(--app-muted)]">
                {bot.description || "Its conversation starts here and continues on every app you pair."}
              </p>
              <div className="mt-5 flex flex-wrap justify-center gap-2">
                {["What are you working on?", "Summarize this project.", "Check the build and tell me what failed."].map((suggestion) => (
                  <button
                    key={suggestion}
                    type="button"
                    className="rounded-full bg-[var(--app-panel)] px-3 py-1.5 text-xs text-[var(--app-muted)] transition hover:bg-[var(--app-panel-2)] hover:text-[var(--app-text)]"
                    onClick={() => {
                      setText(suggestion);
                      composer.current?.focus();
                    }}
                  >
                    {suggestion}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            shown.map((entry) => (
              <ChatMessage
                key={entry.id}
                entry={entry}
                bot={bot}
                authorName={entry.data.author ? (bots.find((item) => item.id === entry.data.author)?.name ?? entry.data.author) : undefined}
                working={working}
                answering={answering}
                onRespond={(id, option) => void respond(id, option)}
              />
            ))
          )}

          {notice ? <p className="text-center text-xs text-[var(--app-faint)]">{notice}</p> : null}
          {working ? (
            <p className="flex items-center justify-center gap-2 text-xs text-[var(--app-faint)]">
              <span className="app-pulse size-1.5 rounded-full bg-[var(--app-good)]" />
              {bot.activity || "Working"}
            </p>
          ) : null}
        </div>
      </div>

      <div className="shrink-0 bg-[var(--app-bg)] px-3 pt-2 pb-[max(0.75rem,env(safe-area-inset-bottom))] sm:px-5">
        <div className="mx-auto w-full max-w-3xl">
          {error ? (
            <div className="mb-2 flex items-start gap-2">
              <div className="min-w-0 flex-1">
                <ErrorNote>{error}</ErrorNote>
              </div>
              <button type="button" className={iconButton()} aria-label="Dismiss" onClick={() => setError(null)}>
                ×
              </button>
            </div>
          ) : null}
          <div className="flex items-end gap-2 rounded-3xl bg-[var(--app-panel)] p-1.5">
            <label className="min-w-0 flex-1">
              <span className="sr-only">Message {bot.name}</span>
              <textarea
                ref={composer}
                value={text}
                rows={1}
                placeholder={`Message ${bot.name}`}
                onChange={(event) => setText(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                    event.preventDefault();
                    void send();
                  }
                }}
                className="max-h-40 min-h-10 w-full resize-none bg-transparent px-3 py-2.5 text-[15px] leading-relaxed text-[var(--app-text)] outline-none placeholder:text-[var(--app-faint)] [field-sizing:content]"
              />
            </label>
            <button
              type="button"
              className={button("primary", "md")}
              aria-label="Send"
              disabled={!text.trim() || sending}
              onClick={() => void send()}
              style={{ width: 40, padding: 0 }}
            >
              <PaperPlaneTilt size={17} weight="fill" />
            </button>
          </div>
          <p className="mt-1.5 text-center text-[10px] text-[var(--app-faint)]">
            Enter sends · Shift+Enter is a new line · messages queue while the bot works
          </p>
        </div>
      </div>
    </div>
  );
}
