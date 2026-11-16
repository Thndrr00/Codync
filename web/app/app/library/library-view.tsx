"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
import {
  ArrowClockwise,
  ArrowSquareOut,
  Books,
  DownloadSimple,
  File as FileIcon,
  FileImage,
  FileText,
} from "@phosphor-icons/react";
import { formatBytes, formatRelative, readableError } from "@/lib/host-protocol";
import type { ServerLibraryItem } from "@/lib/host-types";
import { useHost } from "../host-provider";
import { Badge, button, EmptyState, ErrorNote, iconButton, PageHeader, Skeleton, useNow } from "../ui";
import ArtifactViewer from "./artifact-viewer";

type Filter = "all" | "images" | "files";

function fileName(item: ServerLibraryItem): string {
  return item.kind === "artifact" ? item.artifact.title : item.file.name;
}

function ItemCard({ item, now, onOpen, onDownload, busy }: {
  item: ServerLibraryItem;
  now: number;
  onOpen: () => void;
  onDownload: () => void;
  busy: boolean;
}) {
  const artifact = item.kind === "artifact";
  const image = item.kind !== "artifact" && item.image;
  const Icon = artifact ? FileText : image ? FileImage : FileIcon;
  const detail =
    artifact
      ? item.artifact.kind === "diagram"
        ? "Diagram"
        : "Artifact"
      : item.kind === "attachment"
        ? "Attachment"
        : "Shared file";

  return (
    <article className="group flex min-h-40 flex-col rounded-3xl bg-[var(--app-panel)] p-4 transition hover:bg-[var(--app-panel-2)]">
      <div className="flex items-start gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-2xl bg-[var(--app-panel-2)] text-[var(--app-muted)]">
          <Icon size={20} />
        </span>
        <div className="min-w-0 flex-1">
          <h2 className="line-clamp-2 text-sm font-semibold leading-snug text-[var(--app-text)]">{fileName(item)}</h2>
          <p className="mt-1 truncate text-xs text-[var(--app-muted)]">{item.botName}</p>
        </div>
      </div>
      <div className="mt-auto flex items-center gap-2 pt-4 text-[11px] text-[var(--app-faint)]">
        <Badge>{detail}</Badge>
        {item.kind !== "artifact" ? <span>{formatBytes(item.file.size)}</span> : null}
        {now > 0 ? <span>{formatRelative(item.at, now)}</span> : null}
        <span className="ml-auto">
          {artifact ? (
            <button type="button" className={button("quiet", "sm")} onClick={onOpen}>
              Open <ArrowSquareOut size={12} />
            </button>
          ) : (
            <button type="button" className={button("quiet", "sm")} disabled={busy} onClick={onDownload}>
              {busy ? "Getting…" : <DownloadSimple size={13} />}
              {busy ? null : "Download"}
            </button>
          )}
        </span>
      </div>
    </article>
  );
}

export default function LibraryView() {
  const params = useSearchParams();
  const botParam = params.get("bot") ?? "";
  const artifactParam = params.get("artifact") ?? "";
  const { client, bots, subscribe, synced } = useHost();
  const [filter, setFilter] = useState<Filter>("all");
  const [items, setItems] = useState<ServerLibraryItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [viewing, setViewing] = useState<{ botId: string; artifactId: string; title: string } | null>(null);
  const now = useNow();
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void client
      .serverLibrary(filter)
      .then((answer) => {
        if (!cancelled) {
          setItems(answer.items);
          setError(null);
        }
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
  }, [client, filter, tick]);

  useEffect(() => {
    return subscribe((event) => {
      if (event.type !== "artifact") return;
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => setTick((value) => value + 1), 500);
    });
  }, [subscribe]);

  useEffect(() => {
    return () => {
      if (timer.current) clearTimeout(timer.current);
    };
  }, []);

  useEffect(() => {
    if (botParam && artifactParam) setViewing({ botId: botParam, artifactId: artifactParam, title: "Artifact" });
  }, [botParam, artifactParam]);

  const refresh = useCallback(() => setTick((value) => value + 1), []);

  const download = useCallback(
    async (item: ServerLibraryItem) => {
      if (item.kind === "artifact") return;
      const key = `${item.kind}:${item.botId}:${item.file.id}`;
      setDownloading(key);
      setError(null);
      try {
        const { name, blob } = await client.downloadLibraryFile(item);
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement("a");
        anchor.href = url;
        anchor.download = name;
        document.body.appendChild(anchor);
        anchor.click();
        anchor.remove();
        setTimeout(() => URL.revokeObjectURL(url), 2_000);
      } catch (failure) {
        setError(readableError(failure));
      } finally {
        setDownloading(null);
      }
    },
    [client],
  );

  const shown = botParam ? items.filter((item) => item.botId === botParam) : items;
  const botName = bots.find((bot) => bot.id === botParam)?.name;

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto w-full max-w-5xl px-4 py-6 sm:px-6 md:py-10">
        <PageHeader
          title="Library"
          subtitle={
            botParam ? (
              <>
                Everything {botName ? <span className="text-[var(--app-text)]">{botName}</span> : "this bot"} published or shared.{" "}
                <Link href="/app/library" prefetch={false} className="text-[var(--app-text)] underline underline-offset-4">
                  Show every bot
                </Link>
              </>
            ) : (
              "Artifacts your bots published and files they shared, from every chat on this computer."
            )
          }
          actions={
            <button type="button" className={iconButton()} aria-label="Refresh" title="Refresh" onClick={refresh}>
              <ArrowClockwise size={18} />
            </button>
          }
        />

        <div className="mt-5 flex flex-wrap items-center gap-2">
          {(
            [
              ["all", "Everything"],
              ["images", "Images"],
              ["files", "Files"],
            ] as const
          ).map(([value, label]) => (
            <button key={value} type="button" aria-pressed={filter === value} className={button(filter === value ? "primary" : "quiet", "sm")} onClick={() => setFilter(value)}>
              {label}
            </button>
          ))}
          <span className="ml-auto text-xs text-[var(--app-faint)]">{shown.length} items</span>
        </div>

        {error ? (
          <div className="mt-4">
            <ErrorNote>{error}</ErrorNote>
          </div>
        ) : null}

        {loading && !shown.length ? (
          <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {[0, 1, 2, 3, 4, 5].map((key) => (
              <Skeleton key={key} className="h-40" />
            ))}
          </div>
        ) : !shown.length ? (
          <div className="mt-5">
            <EmptyState icon={<Books size={24} />} title={synced ? "Nothing here yet" : "Loading the Library"}>
              When a bot publishes an artifact or shares a file, it lands here — for every chat, newest first.
            </EmptyState>
          </div>
        ) : (
          <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {shown.map((item, index) => {
              const key =
                item.kind === "artifact" ? `a:${item.artifact.id}` : `${item.kind}:${item.botId}:${item.file.id}:${index}`;
              const downloadKey = item.kind === "artifact" ? "" : `${item.kind}:${item.botId}:${item.file.id}`;
              return (
                <ItemCard
                  key={key}
                  item={item}
                  now={now}
                  busy={downloading === downloadKey}
                  onOpen={() =>
                    item.kind === "artifact" &&
                    setViewing({ botId: item.botId, artifactId: item.artifact.id, title: item.artifact.title })
                  }
                  onDownload={() => void download(item)}
                />
              );
            })}
          </div>
        )}

        <p className="mt-8 text-xs leading-relaxed text-[var(--app-faint)]">
          Artifacts open in a sandboxed frame: they can run their own scripts, but can&apos;t read this page, your token or the rest of Codync.
          Diagram editing and files over 64 MB stay in the desktop app.
        </p>
      </div>

      {viewing ? (
        <ArtifactViewer
          botId={viewing.botId}
          artifactId={viewing.artifactId}
          title={viewing.title}
          onClose={() => setViewing(null)}
        />
      ) : null}
    </div>
  );
}
