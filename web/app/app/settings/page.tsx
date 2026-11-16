"use client";

import { useState } from "react";
import Link from "next/link";
import {
  Check,
  Copy,
  Desktop,
  GearSix,
  Lock,
  Monitor,
  Moon,
  Sun,
} from "@phosphor-icons/react";
import { readableError } from "@/lib/host-protocol";
import { useHost } from "../host-provider";
import { useTheme, type Theme } from "../theme";
import { button, ErrorNote, iconButton, PageHeader, surface } from "../ui";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-wrap items-baseline justify-between gap-2 py-2.5">
      <dt className="text-sm text-[var(--app-muted)]">{label}</dt>
      <dd className="min-w-0 max-w-full text-right text-sm text-[var(--app-text)]">{children}</dd>
    </div>
  );
}

function Switch({ on, onChange, label }: { on: boolean; onChange: (next: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
      className={`relative h-6 w-10 shrink-0 rounded-full transition ${on ? "bg-[var(--app-good)]" : "bg-[var(--app-panel-2)]"}`}
    >
      <span aria-hidden className={`absolute top-1 size-4 rounded-full bg-[var(--app-bg)] transition-all ${on ? "left-5" : "left-1"}`} />
    </button>
  );
}

export default function SettingsPage() {
  const { hello, client, connection, refresh } = useHost();
  const { theme, setTheme } = useTheme();
  const [analytics, setAnalytics] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const shownAnalytics = analytics ?? hello?.analytics ?? true;
  const current = hello ? `v${hello.version}` : "—";
  const latest = hello?.latestVersion ? `v${hello.latestVersion}` : null;

  const themes: { id: Theme; label: string; icon: typeof Sun }[] = [
    { id: "system", label: "System", icon: Monitor },
    { id: "light", label: "Light", icon: Sun },
    { id: "dark", label: "Dark", icon: Moon },
  ];

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto w-full max-w-3xl px-4 py-6 sm:px-6 md:py-10">
        <PageHeader title="Settings" subtitle="This browser's connection, how the app looks, and what stays in the desktop app." />

        {error ? (
          <div className="mt-4">
            <ErrorNote>{error}</ErrorNote>
          </div>
        ) : null}

        <section className="mt-6">
          <h2 className="mb-2 px-1 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">Appearance</h2>
          <div className={`${surface} p-4`}>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-sm font-medium text-[var(--app-text)]">Theme</p>
                <p className="mt-0.5 text-xs text-[var(--app-muted)]">Follow your system, or pick one for this browser.</p>
              </div>
              <div className="flex items-center gap-1 rounded-full bg-[var(--app-panel-2)] p-1">
                {themes.map(({ id, label, icon: Icon }) => (
                  <button
                    key={id}
                    type="button"
                    aria-pressed={theme === id}
                    onClick={() => setTheme(id)}
                    className={`inline-flex h-8 items-center gap-1.5 rounded-full px-3 text-xs font-medium transition ${
                      theme === id ? "bg-[var(--app-accent)] text-[var(--app-accent-text)]" : "text-[var(--app-muted)] hover:text-[var(--app-text)]"
                    }`}
                  >
                    <Icon size={13} />
                    {label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </section>

        <section className="mt-6">
          <h2 className="mb-2 px-1 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">This computer</h2>
          <div className={`${surface} px-4 py-2`}>
            <dl className="divide-y divide-[var(--app-line)]">
              <Row label="Computer">
                <span className="inline-flex items-center gap-2">
                  <Desktop size={15} className="text-[var(--app-faint)]" />
                  {hello?.name || "—"}
                </span>
              </Row>
              <Row label="Host">
                <span className="font-mono text-xs">{current}</span>
                {latest && latest !== current ? <span className="ml-2 text-xs text-[var(--app-warn)]">{latest} available</span> : null}
              </Row>
              <Row label="System">
                <span className="text-xs">{hello?.os || "—"}</span>
              </Row>
              <Row label="Addresses">
                <span className="font-mono text-xs break-all">{(hello?.urls ?? []).slice(0, 3).join("  ·  ") || "—"}</span>
              </Row>
              <Row label="Computer ID">
                <span className="inline-flex items-center gap-1.5">
                  <span className="max-w-[12rem] truncate font-mono text-xs">{hello?.computerId || "—"}</span>
                  <button
                    type="button"
                    className={iconButton("!size-7 !px-0")}
                    aria-label="Copy the computer ID"
                    onClick={() => {
                      if (!hello?.computerId) return;
                      void navigator.clipboard
                        ?.writeText(hello.computerId)
                        .then(() => {
                          setCopied(true);
                          setTimeout(() => setCopied(false), 1500);
                        })
                        .catch(() => {});
                    }}
                  >
                    {copied ? <Check size={13} /> : <Copy size={13} />}
                  </button>
                </span>
              </Row>
            </dl>
          </div>
        </section>

        <section className="mt-6">
          <h2 className="mb-2 px-1 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">Privacy</h2>
          <div className={`${surface} p-4`}>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="min-w-0 max-w-[28rem]">
                <p className="text-sm font-medium text-[var(--app-text)]">Product analytics</p>
                <p className="mt-0.5 text-xs leading-relaxed text-[var(--app-muted)]">
                  Anonymous counts of features used, on every app. Off never sends anything. This is the same switch as in the desktop app.
                </p>
              </div>
              <Switch
                on={shownAnalytics}
                label="Product analytics"
                onChange={(next) => {
                  setSaving(true);
                  setError(null);
                  void client
                    .call<{ enabled: boolean }>("setAnalytics", { enabled: next })
                    .then((answer) => {
                      setAnalytics(answer.enabled);
                      refresh();
                    })
                    .catch((failure) => setError(readableError(failure)))
                    .finally(() => setSaving(false));
                }}
              />
            </div>
            {saving ? <p className="mt-2 text-xs text-[var(--app-faint)]">Saving…</p> : null}
          </div>
        </section>

        <section className="mt-6">
          <h2 className="mb-2 px-1 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">Connection</h2>
          <div className={`${surface} p-4`}>
            <div className="flex items-start gap-2.5">
              <Lock size={16} className="mt-0.5 shrink-0 text-[var(--app-muted)]" />
              <div className="min-w-0 text-xs leading-relaxed text-[var(--app-muted)]">
                <p>
                  {connection === "online"
                    ? "This page is served by the same-origin bridge on your computer. The host token stays in the bridge process; the browser never receives it, and no token is ever put in a URL."
                    : "The bridge isn't reachable. Start it from the connection screen."}
                </p>
                <p className="mt-2">
                  Phone pairing uses Codync&apos;s own end-to-end encrypted channel and can&apos;t be done from a browser.{" "}
                  <Link href="/docs/web-app" prefetch={false} className="text-[var(--app-text)] underline underline-offset-4">
                    How the web app connects
                  </Link>
                  .
                </p>
              </div>
            </div>
          </div>
        </section>

        <section className="mt-6">
          <h2 className="mb-2 px-1 text-[11px] font-medium tracking-[0.18em] text-[var(--app-faint)] uppercase">Desktop only</h2>
          <div className={`${surface} p-4`}>
            <p className="text-sm leading-relaxed text-[var(--app-muted)]">
              Remote screen and computer control, the Dynamic Island and menu-bar presence, local SSH profiles, the agent setup terminals, the diagram
              editor, host updates and system permissions belong to the desktop app. The web client intentionally doesn&apos;t fake them.
            </p>
            <Link href="/download" prefetch={false} className={`${button("quiet", "sm")} mt-3`}>
              <GearSix size={13} /> Get the desktop app
            </Link>
          </div>
        </section>

        <p className="mt-8 text-xs leading-relaxed text-[var(--app-faint)]">
          Bots, chats, routines, memory and artifacts stay on this computer. Closing the tab changes nothing; the host keeps working.
        </p>
      </div>
    </div>
  );
}
