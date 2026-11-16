"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Books,
  Brain,
  ClockCountdown,
  GearSix,
  Moon,
  Plugs,
  Robot,
  Sun,
} from "@phosphor-icons/react";
import { HostGate } from "./connection";
import { useHost } from "./host-provider";
import { useTheme } from "./theme";
import { iconButton } from "./ui";

const nav = [
  { href: "/app", label: "Bots", icon: Robot },
  { href: "/app/routines", label: "Routines", icon: ClockCountdown },
  { href: "/app/library", label: "Library", icon: Books },
  { href: "/app/memory", label: "Memory", icon: Brain },
  { href: "/app/settings", label: "Settings", icon: GearSix },
] as const;

function active(pathname: string, href: string) {
  return href === "/app" ? pathname === "/app" || pathname === "/app/" : pathname === href || pathname.startsWith(`${href}/`);
}

function ConnectionCard() {
  const { connection, hello } = useHost();
  const label = connection === "online" ? "Connected" : connection === "connecting" ? "Connecting" : "Not connected";
  const color = connection === "online" ? "var(--app-good)" : connection === "connecting" ? "var(--app-warn)" : "var(--app-bad)";
  return (
    <div className="rounded-2xl bg-[var(--app-panel)] px-4 py-3">
      <div className="flex items-center gap-2">
        <span aria-hidden className="size-2 rounded-full" style={{ background: color }} />
        <span className="text-xs font-medium text-[var(--app-text)]">{label}</span>
        <Plugs size={13} className="ml-auto text-[var(--app-faint)]" />
      </div>
      <p className="mt-1 truncate text-xs text-[var(--app-faint)]" title={hello?.name}>
        {connection === "online" ? (hello?.name || "Your computer") : "Open this on your computer"}
      </p>
    </div>
  );
}

function ThemeButton() {
  const { resolved, setTheme } = useTheme();
  return (
    <button
      type="button"
      className={iconButton()}
      aria-label={resolved === "dark" ? "Use the light theme" : "Use the dark theme"}
      title={resolved === "dark" ? "Light theme" : "Dark theme"}
      onClick={() => setTheme(resolved === "dark" ? "light" : "dark")}
    >
      {resolved === "dark" ? <Sun size={18} /> : <Moon size={18} />}
    </button>
  );
}

function AppShellBody({ children, pathname }: { children: React.ReactNode; pathname: string }) {
  const { connection, hello, refresh } = useHost();
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      {hello && connection !== "online" ? (
        <p
          role="status"
          className="mx-4 mt-2 flex shrink-0 items-center justify-center gap-3 rounded-full bg-[var(--app-panel)] px-3 py-1.5 text-xs text-[var(--app-warn)]"
        >
          {connection === "connecting" ? "Reconnecting to your computer…" : "Lost the connection to your computer."}
          {connection === "offline" ? (
            <button type="button" onClick={refresh} className="font-medium text-[var(--app-text)] underline underline-offset-2">
              Retry
            </button>
          ) : null}
        </p>
      ) : null}

      <header className="flex h-14 shrink-0 items-center gap-3 bg-[var(--app-bg)] px-4 md:hidden">
        <Link href="/app" prefetch={false} className="flex items-center gap-2 font-semibold text-[var(--app-text)]">
          <Image src="/icon.png" alt="" width={24} height={24} className="rounded-[6px]" />
          Codync
        </Link>
        <div className="ml-auto flex items-center gap-1">
          <ThemeButton />
        </div>
      </header>

      <main id="web-app-main" className="min-h-0 flex-1 overflow-hidden">
        <HostGate>{children}</HostGate>
      </main>

      <nav aria-label="Web app" className="flex shrink-0 justify-around bg-[var(--app-bg)] px-2 pt-1 pb-[max(0.5rem,env(safe-area-inset-bottom))] md:hidden">
        {nav.map(({ href, label, icon: Icon }) => {
          const current = active(pathname, href);
          return (
            <Link
              key={href}
              href={href}
              prefetch={false}
              aria-current={current ? "page" : undefined}
              className={`flex min-w-14 flex-col items-center gap-1 rounded-2xl px-2 py-1.5 text-[10px] transition ${
                current ? "text-[var(--app-text)]" : "text-[var(--app-faint)]"
              }`}
            >
              <Icon size={21} weight={current ? "fill" : "regular"} />
              {label}
            </Link>
          );
        })}
      </nav>
    </div>
  );
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  return (
    <div className="app-shell flex h-dvh flex-col overflow-hidden bg-[var(--app-bg)] text-[var(--app-text)] md:flex-row">
      <a
        href="#web-app-main"
        className="sr-only z-50 rounded-full bg-[var(--app-accent)] px-4 py-2 text-sm font-medium text-[var(--app-accent-text)] focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
      >
        Skip to content
      </a>

      <aside className="hidden w-60 shrink-0 flex-col px-3 py-4 md:flex">
        <Link href="/app" prefetch={false} className="flex items-center gap-2.5 rounded-full px-3 py-2 font-semibold text-[var(--app-text)]">
          <Image src="/icon.png" alt="" width={26} height={26} className="rounded-[7px]" />
          Codync
          <span className="ml-auto rounded-full bg-[var(--app-panel-2)] px-2 py-0.5 text-[10px] font-medium text-[var(--app-muted)]">web</span>
        </Link>

        <nav aria-label="Web app" className="mt-6 flex flex-col gap-1">
          {nav.map(({ href, label, icon: Icon }) => {
            const current = active(pathname, href);
            return (
              <Link
                key={href}
                href={href}
                prefetch={false}
                aria-current={current ? "page" : undefined}
                className={`flex h-10 items-center gap-3 rounded-full px-3 text-sm transition ${
                  current
                    ? "bg-[var(--app-panel)] font-medium text-[var(--app-text)]"
                    : "text-[var(--app-muted)] hover:bg-[var(--app-panel)] hover:text-[var(--app-text)]"
                }`}
              >
                <Icon size={18} weight={current ? "fill" : "regular"} />
                {label}
              </Link>
            );
          })}
        </nav>

        <div className="mt-auto flex flex-col gap-3 pt-6">
          <ConnectionCard />
          <div className="flex items-center justify-between px-1">
            <Link href="/docs/web-app" prefetch={false} className="text-xs text-[var(--app-faint)] transition hover:text-[var(--app-text)]">
              How this works
            </Link>
            <ThemeButton />
          </div>
        </div>
      </aside>

      <AppShellBody pathname={pathname}>{children}</AppShellBody>
    </div>
  );
}
