"use client";

import Link from "next/link";
import { ArrowUpRight, Check, Paperclip, WarningCircle } from "@phosphor-icons/react";
import { formatTime } from "@/lib/host-protocol";
import type { Bot, Entry, PermissionOption } from "@/lib/host-types";
import { Badge, BotAvatar, button, well } from "../ui";

function OptionButton({ option, disabled, onClick }: { option: PermissionOption; disabled: boolean; onClick: () => void }) {
  const variant = option.kind.startsWith("reject") ? "danger" : option.kind === "allow_always" ? "quiet" : "primary";
  return (
    <button type="button" className={button(variant, "sm")} disabled={disabled} onClick={onClick}>
      {option.kind.startsWith("allow") ? <Check size={14} weight="bold" /> : null}
      {option.name}
    </button>
  );
}

function PermissionCard({ entry, busy, onRespond }: { entry: Entry; busy: boolean; onRespond: (optionId: string) => void }) {
  const options = entry.data.options ?? [];
  const pending = (entry.data.status ?? "pending") === "pending";
  const selected = options.find((option) => option.optionId === entry.data.selected);
  return (
    <article className="rounded-3xl bg-[var(--app-panel)] p-4">
      <div className="flex items-start gap-2.5">
        <WarningCircle size={18} className="mt-0.5 shrink-0 text-[var(--app-warn)]" />
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-semibold text-[var(--app-text)]">{entry.data.title || "Approval needed"}</h3>
          {entry.data.detail ? <p className="mt-1 text-sm leading-relaxed text-[var(--app-muted)]">{entry.data.detail}</p> : null}
          {entry.data.command ? (
            <pre className={`${well} mt-3 overflow-x-auto p-3 font-mono text-xs leading-relaxed text-[var(--app-text)]`}>
              <code>{entry.data.command}</code>
            </pre>
          ) : null}
          {pending ? (
            <div className="mt-3 flex flex-wrap gap-2">
              {options.map((option) => (
                <OptionButton key={option.optionId} option={option} disabled={busy} onClick={() => onRespond(option.optionId)} />
              ))}
            </div>
          ) : (
            <div className="mt-3 flex items-center gap-2 text-xs text-[var(--app-muted)]">
              <Badge tone={entry.data.status === "answered" ? "good" : "quiet"}>
                {entry.data.status === "answered" ? "Answered" : entry.data.status || "Closed"}
              </Badge>
              {selected ? <span>{selected.name}</span> : null}
            </div>
          )}
        </div>
      </div>
    </article>
  );
}

function SubagentCard({ entry }: { entry: Entry }) {
  const agents = entry.data.subagents ?? [];
  return (
    <article className={`${well} p-3`}>
      <p className="text-xs font-medium text-[var(--app-muted)]">
        {agents.length} {agents.length === 1 ? "subagent" : "subagents"}
      </p>
      <ul className="mt-2 space-y-1.5">
        {agents.slice(0, 6).map((agent) => (
          <li key={agent.id} className="flex items-baseline gap-2 text-xs">
            <span className="text-[var(--app-faint)]">{agent.state}</span>
            <span className="min-w-0 truncate text-[var(--app-text)]">{agent.task || agent.step}</span>
          </li>
        ))}
      </ul>
    </article>
  );
}

/** One chat line: a message, an approval card, a subagent summary or a notice. */
export function ChatMessage({
  entry,
  bot,
  authorName,
  working,
  answering,
  onRespond,
}: {
  entry: Entry;
  bot: Bot;
  /** The group member that wrote it, when the host names one. */
  authorName?: string;
  working: boolean;
  answering: boolean;
  onRespond: (entryId: string, optionId: string) => void;
}) {
  if (entry.kind === "permission") {
    return <PermissionCard entry={entry} busy={answering} onRespond={(optionId) => onRespond(entry.id, optionId)} />;
  }
  if (entry.kind === "subagents") return <SubagentCard entry={entry} />;
  if (entry.kind === "notice") {
    return (
      <p className={`px-4 text-center text-xs leading-relaxed ${entry.data.style === "error" ? "text-[var(--app-bad)]" : "text-[var(--app-faint)]"}`}>
        {entry.data.text}
      </p>
    );
  }

  const mine = entry.kind === "user";
  const text = entry.data.text ?? "";
  const speaker = authorName || bot.name;
  const streaming = !mine && entry.data.final !== true && working;
  const artifact = entry.data.artifact;
  const files = [...(entry.data.attachments ?? []), ...(entry.data.files ?? [])];

  return (
    <article className={`flex gap-2.5 ${mine ? "justify-end" : ""}`}>
      {!mine ? <BotAvatar bot={bot} size={30} /> : null}
      <div className={`flex min-w-0 max-w-[88%] flex-col gap-1.5 sm:max-w-[78%] ${mine ? "items-end" : "items-start"}`}>
        {entry.data.replyTo ? (
          <div className="max-w-full rounded-2xl bg-[var(--app-panel-2)] px-3 py-1.5 text-xs text-[var(--app-muted)]">
            <span className="font-medium text-[var(--app-text)]">{entry.data.replyTo.author === "user" ? "You" : entry.data.replyTo.author}</span>
            <span className="ml-2 line-clamp-1">{entry.data.replyTo.text}</span>
          </div>
        ) : null}
        {!mine && speaker !== bot.name ? <span className="px-1 text-[11px] font-medium text-[var(--app-muted)]">{speaker}</span> : null}
        {text ? (
          <div
            className={`rounded-3xl px-4 py-2.5 text-[15px] leading-relaxed break-words whitespace-pre-wrap ${
              mine
                ? "rounded-br-lg bg-[var(--app-accent)] text-[var(--app-accent-text)]"
                : "rounded-bl-lg bg-[var(--app-panel)] text-[var(--app-text)]"
            }`}
          >
            {text}
            {streaming ? <span aria-label="Still writing" className="app-caret" /> : null}
          </div>
        ) : null}
        {artifact ? (
          <Link
            href={`/app/library?bot=${encodeURIComponent(bot.id)}&artifact=${encodeURIComponent(artifact.id)}`}
            prefetch={false}
            className="inline-flex max-w-full items-center gap-2 rounded-full bg-[var(--app-panel-2)] px-3 py-1.5 text-xs text-[var(--app-text)] transition hover:bg-[var(--app-hover)]"
          >
            <Paperclip size={13} />
            <span className="truncate">{artifact.title}</span>
            <ArrowUpRight size={12} className="shrink-0 text-[var(--app-faint)]" />
          </Link>
        ) : null}
        {files.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {files.map((file) => (
              <span key={file.id} className="inline-flex items-center gap-1.5 rounded-full bg-[var(--app-panel-2)] px-2.5 py-1 text-xs text-[var(--app-muted)]">
                <Paperclip size={12} />
                <span className="max-w-[12rem] truncate">{file.name}</span>
              </span>
            ))}
          </div>
        ) : null}
        <span className="px-1 text-[10px] text-[var(--app-faint)]">
          {mine ? "You" : speaker} · {formatTime(entry.createdAt)}
        </span>
      </div>
    </article>
  );
}
