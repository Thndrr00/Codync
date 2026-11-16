"use client";

import Link from "next/link";
import type { Bot } from "@/lib/host-types";
import { Badge, BotAvatar, StatusDot } from "../ui";

/** Chat's landing view when the URL names no bot. */
export default function BotChooser({ bots }: { bots: Bot[] }) {
  const visible = bots.filter((bot) => !bot.hidden && !bot.archived);
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto w-full max-w-2xl px-4 py-10 sm:px-6">
        <h1 className="text-2xl font-semibold tracking-tight text-[var(--app-text)]">Chat</h1>
        <p className="mt-2 text-sm text-[var(--app-muted)]">Pick a bot to open its conversation.</p>
        <div className="mt-6 space-y-2">
          {visible.map((bot) => (
            <Link
              key={bot.id}
              href={`/app/chat?bot=${encodeURIComponent(bot.id)}`}
              prefetch={false}
              className="flex items-center gap-3 rounded-2xl bg-[var(--app-panel)] px-4 py-3 transition hover:bg-[var(--app-panel-2)]"
            >
              <BotAvatar bot={bot} size={34} />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium text-[var(--app-text)]">{bot.name}</span>
                <span className="mt-0.5 flex items-center gap-1.5 text-xs text-[var(--app-muted)]">
                  <StatusDot status={bot.status} size={6} />
                  {bot.status === "working" ? bot.activity || "Working" : bot.description || "Idle"}
                </span>
              </span>
              {bot.unread > 0 ? <Badge>{bot.unread}</Badge> : null}
            </Link>
          ))}
          {!visible.length ? (
            <p className="rounded-2xl bg-[var(--app-panel)] px-4 py-6 text-center text-sm text-[var(--app-muted)]">
              No bots on this computer yet. Create one in the desktop app or on your phone, then open it here.
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
