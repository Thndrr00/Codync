"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { ArrowSquareOut, Check, Copy, Plugs, TerminalWindow } from "@phosphor-icons/react";
import { useHost } from "./host-provider";
import { button, ErrorNote, iconButton, surface, well } from "./ui";

const COMMAND = "cd web\nnode proxy/host-proxy.mjs";

function CopyCommand() {
  const [copied, setCopied] = useState(false);
  return (
    <div className={`${well} relative mt-3`}>
      <pre className="overflow-x-auto p-4 pr-14 font-mono text-[13px] leading-relaxed text-[var(--app-text)]">
        <code>{COMMAND}</code>
      </pre>
      <button
        type="button"
        aria-label="Copy the command"
        className={iconButton("absolute top-2 right-2 bg-[var(--app-panel)]")}
        onClick={() => {
          void navigator.clipboard
            ?.writeText(COMMAND)
            .then(() => {
              setCopied(true);
              setTimeout(() => setCopied(false), 1600);
            })
            .catch(() => {});
        }}
      >
        {copied ? <Check size={16} /> : <Copy size={16} />}
      </button>
    </div>
  );
}

export function HostGate({ children }: { children: React.ReactNode }) {
  const { connection, hello } = useHost();
  // Keep a screen mounted through a reconnect: a blip must not throw away a half-typed message.
  if (connection === "online" || hello) return <>{children}</>;
  return <ConnectionPanel />;
}

function ConnectionPanel() {
  const { connection, error, hello, refresh } = useHost();
  const [retrying, setRetrying] = useState(false);
  const [origin, setOrigin] = useState("");
  let hosted = false;
  if (origin) {
    try {
      hosted = !["localhost", "127.0.0.1", "[::1]"].includes(new URL(origin).hostname);
    } catch {
      hosted = false;
    }
  }

  useEffect(() => {
    setOrigin(window.location.origin);
  }, []);

  if (connection === "connecting") {
    return (
      <div className="h-full overflow-y-auto">
        <div className="mx-auto flex min-h-[60vh] max-w-md flex-col items-center justify-center px-6 text-center">
          <span className="app-pulse flex size-14 items-center justify-center rounded-full bg-[var(--app-panel)] text-[var(--app-text)]">
            <Plugs size={26} />
          </span>
          <h1 className="mt-6 text-xl font-semibold tracking-tight text-[var(--app-text)]">Connecting to your computer</h1>
          <p className="mt-2 text-sm leading-relaxed text-[var(--app-muted)]">Looking for the Codync host on this origin.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto w-full max-w-2xl px-4 py-10 sm:px-6 md:py-16">
        <div className={`${surface} overflow-hidden p-6 sm:p-8`}>
        <span className="flex size-12 items-center justify-center rounded-full bg-[var(--app-panel-2)] text-[var(--app-text)]">
          <TerminalWindow size={24} />
        </span>
        <h1 className="mt-5 text-2xl font-semibold tracking-tight text-balance text-[var(--app-text)]">
          {hosted ? "Open the web app from your own computer." : "Start the local bridge."}
        </h1>
        <p className="mt-3 max-w-[36rem] text-sm leading-relaxed text-[var(--app-muted)]">
          {hosted
            ? "You're on the Codync website. Your bots live on your computer, and this page can't reach it from here. Run the bridge below, then open the address it prints."
            : "The web app talks to your computer's host, which accepts only its own loopback origin. The bridge serves this page and adds the host token on the way, so the browser never holds it."}
        </p>

        {error ? (
          <div className="mt-5">
            <ErrorNote>{error}</ErrorNote>
          </div>
        ) : null}

        <ol className="mt-6 space-y-5">
          <li>
            <p className="text-sm font-medium text-[var(--app-text)]">1. On the computer running Codync, run</p>
            <CopyCommand />
            <p className="mt-2 text-xs leading-relaxed text-[var(--app-faint)]">
              It serves the built client from <code className="font-mono">web/out</code>. Run <code className="font-mono">npm run build</code> in{" "}
              <code className="font-mono">web/</code> first if you haven&apos;t.
            </p>
          </li>
          <li>
            <p className="text-sm font-medium text-[var(--app-text)]">2. Open the address it prints</p>
            <p className={`${well} mt-3 inline-block px-3 py-2 font-mono text-[13px] text-[var(--app-text)]`}>http://localhost:4321</p>
            <p className="mt-2 text-xs leading-relaxed text-[var(--app-faint)]">
              Loopback only. Anyone who can open that address can use your bots, so don&apos;t expose it to other devices.
            </p>
          </li>
          <li>
            <p className="text-sm font-medium text-[var(--app-text)]">3. If this screen remains</p>
            <p className="mt-1 text-sm leading-relaxed text-[var(--app-muted)]">
              Check that the Codync desktop app (or <code className="font-mono text-[var(--app-text)]">codync-host serve</code>) is running, then try again.
              {hello ? ` The host answered as ${hello.name || "this computer"}.` : ""}
            </p>
          </li>
        </ol>

        <div className="mt-7 flex flex-wrap items-center gap-3">
          <button
            type="button"
            className={button("primary")}
            disabled={retrying}
            onClick={() => {
              setRetrying(true);
              refresh();
              setTimeout(() => setRetrying(false), 1200);
            }}
          >
            {retrying ? "Trying…" : "Try again"}
          </button>
          <Link href="/docs/web-app" className={button("quiet")}>
            How the web app connects <ArrowSquareOut size={14} />
          </Link>
        </div>

        <p className="mt-6 text-xs leading-relaxed text-[var(--app-faint)]">
          This origin: <span className="font-mono">{origin || "this page"}</span>. Phone pairing is a separate end-to-end
          encrypted channel; a browser can&apos;t join it, which is why the bridge exists.
        </p>
        </div>
      </div>
    </div>
  );
}
