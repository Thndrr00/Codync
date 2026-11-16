"use client";

import Image from "next/image";
import { useEffect, useState } from "react";
import { UsersThree } from "@phosphor-icons/react";
import { CharacterAvatar } from "../components/live-phone/avatar";
import { useTheme } from "./theme";
import type { Bot } from "@/lib/host-types";

export const surface = "rounded-3xl bg-[var(--app-panel)]";
export const well = "rounded-2xl bg-[var(--app-panel-2)]";

const sizes = {
  sm: "h-8 px-3 text-xs",
  md: "h-10 px-4 text-sm",
} as const;

export function button(variant: "primary" | "quiet" | "danger" = "primary", size: keyof typeof sizes = "md") {
  const tone =
    variant === "primary"
      ? "bg-[var(--app-accent)] text-[var(--app-accent-text)] hover:opacity-90"
      : variant === "danger"
        ? "text-[var(--app-bad)] hover:bg-[var(--app-panel-2)]"
        : "bg-[var(--app-panel-2)] text-[var(--app-text)] hover:bg-[var(--app-hover)]";
  return `inline-flex shrink-0 items-center justify-center gap-2 rounded-full font-medium transition active:scale-[0.98] disabled:pointer-events-none disabled:opacity-40 ${tone} ${sizes[size]}`;
}

export function iconButton(extra = "") {
  return `inline-flex size-9 shrink-0 items-center justify-center rounded-full text-[var(--app-muted)] transition hover:bg-[var(--app-panel-2)] hover:text-[var(--app-text)] active:scale-95 ${extra}`;
}

export function StatusDot({ status, size = 8 }: { status: string; size?: number }) {
  const color =
    status === "working"
      ? "var(--app-good)"
      : status === "needsInput"
        ? "var(--app-warn)"
        : status === "error"
          ? "var(--app-bad)"
          : "var(--app-faint)";
  return (
    <span
      aria-hidden
      className={`inline-block shrink-0 rounded-full ${status === "working" || status === "needsInput" ? "app-pulse" : ""}`}
      style={{ width: size, height: size, background: color }}
    />
  );
}

export function Badge({ children, tone = "quiet" }: { children: React.ReactNode; tone?: "quiet" | "good" | "warn" | "bad" }) {
  const color =
    tone === "good"
      ? "text-[var(--app-good)]"
      : tone === "warn"
        ? "text-[var(--app-warn)]"
        : tone === "bad"
          ? "text-[var(--app-bad)]"
          : "text-[var(--app-muted)]";
  return <span className={`inline-flex items-center gap-1 rounded-full bg-[var(--app-panel-2)] px-2 py-0.5 text-[11px] font-medium ${color}`}>{children}</span>;
}

/** The bot's halftone face, its photo, or a group mark. */
export function BotAvatar({ bot, members = [], size = 40 }: { bot: Bot; members?: Bot[]; size?: number }) {
  const { resolved } = useTheme();
  if (bot.kind === "group" && members.length >= 2) {
    return (
      <div className="relative shrink-0" style={{ width: size, height: size }} aria-hidden>
        <div className="absolute top-0 right-0">
          <BotAvatar bot={members[1]} size={Math.round(size * 0.66)} />
        </div>
        <div className="absolute bottom-0 left-0">
          <BotAvatar bot={members[0]} size={Math.round(size * 0.66)} />
        </div>
      </div>
    );
  }
  if (bot.kind === "group") {
    return (
      <div
        className="flex shrink-0 items-center justify-center rounded-[30%] bg-[var(--app-panel-2)] text-[var(--app-muted)]"
        style={{ width: size, height: size }}
        aria-hidden
      >
        <UsersThree size={Math.round(size * 0.5)} />
      </div>
    );
  }
  if (bot.avatarPhoto) {
    return <Image src={bot.avatarPhoto} alt="" width={size} height={size} unoptimized className="shrink-0 rounded-[34%] object-cover" />;
  }
  return (
    <CharacterAvatar
      shape={bot.avatarShape}
      color={bot.avatarColor}
      size={size}
      mood={bot.status === "needsInput" ? "needs" : bot.status === "working" ? "working" : "idle"}
      ink={resolved === "dark" ? "#ededed" : "#141414"}
    />
  );
}

export function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  actions?: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-end justify-between gap-4">
      <div className="min-w-0">
        <h1 className="text-2xl font-semibold tracking-tight text-balance text-[var(--app-text)] md:text-3xl">{title}</h1>
        {subtitle ? <div className="mt-2 max-w-[42rem] text-sm leading-relaxed text-[var(--app-muted)]">{subtitle}</div> : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
    </div>
  );
}

export function EmptyState({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex flex-col items-center justify-center rounded-3xl bg-[var(--app-panel)] px-6 py-16 text-center">
      <div className="flex size-12 items-center justify-center rounded-full bg-[var(--app-panel-2)] text-[var(--app-muted)]">{icon}</div>
      <h2 className="mt-5 text-lg font-semibold text-[var(--app-text)]">{title}</h2>
      {children ? <div className="mt-2 max-w-[30rem] text-sm leading-relaxed text-[var(--app-muted)]">{children}</div> : null}
    </div>
  );
}

export function ErrorNote({ children }: { children: React.ReactNode }) {
  return (
    <p role="alert" className="rounded-2xl bg-[var(--app-panel-2)] px-4 py-3 text-sm leading-relaxed text-[var(--app-bad)]">
      {children}
    </p>
  );
}

export function Spinner({ label = "Loading" }: { label?: string }) {
  return (
    <span className="inline-flex items-center gap-2 text-sm text-[var(--app-muted)]">
      <span
        aria-hidden
        className="size-4 animate-spin rounded-full border-2 border-current border-t-transparent opacity-60"
      />
      {label}
    </span>
  );
}

export function Skeleton({ className = "" }: { className?: string }) {
  return <div className={`animate-pulse rounded-2xl bg-[var(--app-panel-2)] ${className}`} />;
}

/** The browser's clock, after mount: server-rendered times would otherwise disagree. 0 until then. */
export function useNow(intervalMs = 60_000): number {
  const [now, setNow] = useState(0);
  useEffect(() => {
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(timer);
  }, [intervalMs]);
  return now;
}

/** A small native select that reads like the rest of the app. */
export function Select({
  value,
  onChange,
  children,
  label,
  className = "",
}: {
  value: string;
  onChange: (value: string) => void;
  children: React.ReactNode;
  label: string;
  className?: string;
}) {
  return (
    <label className={`relative inline-flex items-center ${className}`}>
      <span className="sr-only">{label}</span>
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-10 max-w-[16rem] appearance-none truncate rounded-full bg-[var(--app-panel-2)] py-0 pr-9 pl-4 text-sm text-[var(--app-text)] outline-none transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--app-text)]"
      >
        {children}
      </select>
      <svg aria-hidden width="10" height="6" viewBox="0 0 10 6" className="pointer-events-none absolute right-4 text-[var(--app-faint)]">
        <path d="M1 1l4 4 4-4" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </label>
  );
}
