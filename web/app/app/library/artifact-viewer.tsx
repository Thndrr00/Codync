"use client";

import { useEffect, useState } from "react";
import { ArrowClockwise, FileHtml, TreeStructure, X } from "@phosphor-icons/react";
import { readableError } from "@/lib/host-protocol";
import type { Artifact } from "@/lib/host-types";
import { useHost } from "../host-provider";
import { button, ErrorNote, iconButton, Skeleton } from "../ui";

export default function ArtifactViewer({
  botId,
  artifactId,
  title,
  onClose,
}: {
  botId: string;
  artifactId: string;
  title: string;
  onClose: () => void;
}) {
  const { client } = useHost();
  const [artifact, setArtifact] = useState<Artifact | null>(null);
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;
    setLoading(true);
    setError(null);
    void client
      .readArtifactHtml(botId, artifactId)
      .then(({ artifact: read, html }) => {
        if (cancelled) return;
        setArtifact(read);
        if (read.kind === "diagram") return;
        objectUrl = URL.createObjectURL(new Blob([html], { type: "text/html" }));
        setUrl(objectUrl);
      })
      .catch((failure) => {
        if (!cancelled) setError(readableError(failure, "The artifact couldn't be opened."));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [artifactId, botId, client, tick]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = previous;
    };
  }, [onClose]);

  const diagram = artifact?.kind === "diagram";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 p-2 backdrop-blur-sm sm:p-6"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={artifact?.title || title}
        className="app-rise flex h-full max-h-[92dvh] w-full max-w-5xl flex-col overflow-hidden rounded-3xl bg-[var(--app-panel)] shadow-[var(--app-shadow)]"
      >
        <header className="flex shrink-0 items-center gap-3 px-4 py-3">
          <span className="flex size-9 items-center justify-center rounded-2xl bg-[var(--app-panel-2)] text-[var(--app-muted)]">
            {diagram ? <TreeStructure size={18} /> : <FileHtml size={18} />}
          </span>
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-sm font-semibold text-[var(--app-text)]">{artifact?.title || title}</h2>
            <p className="truncate text-xs text-[var(--app-muted)]">
              {diagram ? "Diagram artifact" : "Artifact"}
              {artifact ? ` · ${new Date(artifact.updatedAt).toLocaleDateString()}` : ""}
            </p>
          </div>
          <button type="button" className={iconButton()} aria-label="Close" onClick={onClose}>
            <X size={18} />
          </button>
        </header>

        <div className="min-h-0 flex-1 bg-white">
          {loading ? (
            <div className="flex h-full items-center justify-center bg-[var(--app-panel)]">
              <Skeleton className="h-40 w-3/4" />
            </div>
          ) : error ? (
            <div className="flex h-full flex-col items-center justify-center gap-4 bg-[var(--app-panel)] p-6">
              <ErrorNote>{error}</ErrorNote>
              <button type="button" className={button("quiet", "sm")} onClick={() => setTick((value) => value + 1)}>
                <ArrowClockwise size={13} /> Try again
              </button>
            </div>
          ) : diagram ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 bg-[var(--app-panel)] p-8 text-center">
              <TreeStructure size={28} className="text-[var(--app-faint)]" />
              <h3 className="text-sm font-medium text-[var(--app-text)]">Open this diagram in the desktop app</h3>
              <p className="max-w-sm text-xs leading-relaxed text-[var(--app-muted)]">
                A diagram is an editable scene, not a page. The web client shows pages; the desktop app has the diagram editor.
              </p>
            </div>
          ) : url ? (
            <iframe
              title={artifact?.title || title}
              src={url}
              sandbox="allow-scripts allow-popups allow-forms"
              referrerPolicy="no-referrer"
              className="h-full w-full border-0 bg-white"
            />
          ) : null}
        </div>

        <footer className="shrink-0 px-4 py-2.5 text-[11px] text-[var(--app-faint)]">
          Sandboxed: the page can run its own scripts but can&apos;t read Codync, this tab or your token.
        </footer>
      </div>
    </div>
  );
}
