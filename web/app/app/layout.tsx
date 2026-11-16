import type { Metadata } from "next";
import { AppShell } from "./app-shell";
import { HostProvider } from "./host-provider";
import { ThemeProvider } from "./theme";
import "./app.css";

export const metadata: Metadata = {
  title: { default: "Web app", template: "%s | Codync web" },
  description:
    "Codync's web client: your bots, chats, approvals, routines, memory and Library in a browser, talking to the host on your own computer.",
  robots: { index: false, follow: true },
};

export default function WebAppLayout({ children }: { children: React.ReactNode }) {
  return (
    <ThemeProvider>
      <HostProvider>
        <AppShell>{children}</AppShell>
      </HostProvider>
    </ThemeProvider>
  );
}
