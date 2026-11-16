import type { Metadata } from "next";
import { Suspense } from "react";
import ChatView from "./chat-view";
import { Skeleton } from "../ui";

export const metadata: Metadata = { title: "Chat" };

export default function ChatPage() {
  return (
    <Suspense
      fallback={
        <div className="h-full overflow-hidden">
          <div className="mx-auto max-w-3xl space-y-3 px-4 py-6">
            <Skeleton className="h-12" />
            <Skeleton className="h-24" />
            <Skeleton className="h-24" />
          </div>
        </div>
      }
    >
      <ChatView />
    </Suspense>
  );
}
