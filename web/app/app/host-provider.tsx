"use client";

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import { HostClient, HostError, type StreamStatus } from "@/lib/host-client";
import type { Bot, Hello, HostEvent } from "@/lib/host-types";

type Connection = "connecting" | "online" | "offline";

type HostValue = {
  client: HostClient;
  hello: Hello | null;
  bots: Bot[];
  connection: Connection;
  error: string | null;
  /** The first catch-up finished: an empty roster now means no bots, not "still loading". */
  synced: boolean;
  refresh: () => void;
  /** Live host events, for a screen that needs more than the roster. */
  subscribe: (listener: (event: HostEvent) => void) => () => void;
};

const HostContext = createContext<HostValue | null>(null);

function upsertBots(current: Bot[], incoming: Bot): Bot[] {
  const index = current.findIndex((bot) => bot.id === incoming.id);
  if (index < 0) return [...current, incoming];
  const next = current.slice();
  next[index] = incoming.rev >= current[index].rev ? incoming : { ...incoming, ...current[index] };
  return next;
}

function loopbackHost(): boolean {
  const host = window.location.hostname;
  return host === "localhost" || host === "127.0.0.1" || host === "[::1]";
}

/** An error that says what to start, not just which status the host returned. */
function setupMessage(error: unknown): string {
  if (error instanceof HostError && !loopbackHost() && error.status !== 0) {
    return "This page came from the Codync website, which can't reach your computer. Run the local bridge in web/proxy and open the address it prints.";
  }
  if (error instanceof HostError) return error.message;
  return "Can't reach your computer. Is the Codync host running?";
}

export function HostProvider({ children }: { children: React.ReactNode }) {
  // Same origin: the supported setup is the local bridge, which also injects the token.
  const [client] = useState(() => new HostClient({ baseUrl: "" }));
  const [hello, setHello] = useState<Hello | null>(null);
  const [bots, setBots] = useState<Bot[]>([]);
  const [connection, setConnection] = useState<Connection>("connecting");
  const [error, setError] = useState<string | null>(null);
  const [synced, setSynced] = useState(false);
  const [session, setSession] = useState(0);
  const listeners = useRef(new Set<(event: HostEvent) => void>());

  const apply = useCallback((event: HostEvent) => {
    switch (event.type) {
      case "bot":
        setBots((current) => upsertBots(current, event.bot));
        break;
      case "botDeleted":
        setBots((current) => current.filter((bot) => bot.id !== event.id));
        break;
      case "hello":
        setSynced(true);
        break;
      default:
        break;
    }
    for (const listener of listeners.current) listener(event);
  }, []);

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    setConnection("connecting");
    setError(null);

    void (async () => {
      try {
        const answer = await client.hello();
        if (cancelled) return;
        setHello(answer);
        setConnection("online");
      } catch (failure) {
        if (cancelled) return;
        setConnection("offline");
        setError(setupMessage(failure));
        return;
      }
      stop = client.events({
        client: "web",
        onEvent: (event) => {
          if (!cancelled) apply(event);
        },
        onStatus: (status: StreamStatus, failure) => {
          if (cancelled) return;
          setConnection(status);
          if (failure) setError(failure.message);
          else if (status === "online") setError(null);
        },
      });
    })();

    return () => {
      cancelled = true;
      stop?.();
    };
  }, [apply, client, session]);

  const refresh = useCallback(() => setSession((value) => value + 1), []);

  const subscribe = useCallback((listener: (event: HostEvent) => void) => {
    listeners.current.add(listener);
    return () => {
      listeners.current.delete(listener);
    };
  }, []);

  const value = useMemo(
    () => ({ client, hello, bots, connection, error, synced, refresh, subscribe }),
    [client, hello, bots, connection, error, synced, refresh, subscribe],
  );
  return <HostContext.Provider value={value}>{children}</HostContext.Provider>;
}

export function useHost(): HostValue {
  const value = useContext(HostContext);
  if (!value) throw new Error("useHost must be used inside HostProvider");
  return value;
}
