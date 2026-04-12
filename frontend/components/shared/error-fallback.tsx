"use client";

import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { AlertCircle } from "lucide-react";

/**
 * Shared error fallback UI used by every route segment's `error.tsx`.
 *
 * Operator-safe: the displayed message comes from `error.message` only
 * (which, for server-component errors in production, is a generic string
 * with a digest — not the original error). We never render `error.stack`
 * or raw server data. Full details go to the server logs and can be
 * cross-referenced via `error.digest`.
 *
 * Retry uses Next 16's `unstable_retry` (renamed from `reset` in earlier
 * Next versions). It re-runs the server component tree for this segment
 * — the correct primitive for transient failures like "API temporarily
 * unreachable".
 */
export function ErrorFallback({
  error,
  unstable_retry,
  title = "Something went wrong",
}: {
  error: Error & { digest?: string };
  unstable_retry: () => void;
  title?: string;
}) {
  useEffect(() => {
    // Log the full error for debugging — only reaches the client console,
    // not the displayed UI.
    console.error("[error boundary]", error);
  }, [error]);

  return (
    <div className="p-6">
      <Card className="max-w-xl border-red-200">
        <CardHeader className="flex flex-row items-center gap-2">
          <AlertCircle className="h-5 w-5 text-red-600" />
          <CardTitle className="text-base text-red-900">{title}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm text-zinc-600">{error.message}</p>
          {error.digest && (
            <p className="font-mono text-xs text-zinc-400">
              digest: {error.digest}
            </p>
          )}
          <Button variant="outline" size="sm" onClick={unstable_retry}>
            Try again
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
