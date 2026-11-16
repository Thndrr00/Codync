import type { Metadata } from "next";
import { Suspense } from "react";
import LibraryView from "./library-view";
import { Skeleton } from "../ui";

export const metadata: Metadata = { title: "Library" };

export default function LibraryPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-5xl px-4 py-8">
          <Skeleton className="h-8 w-40" />
          <div className="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {[0, 1, 2, 3].map((key) => (
              <Skeleton key={key} className="h-40" />
            ))}
          </div>
        </div>
      }
    >
      <LibraryView />
    </Suspense>
  );
}
