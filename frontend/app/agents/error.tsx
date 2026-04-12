"use client";

import { ErrorFallback } from "@/components/shared/error-fallback";

export default function AgentsError({
  error,
  unstable_retry,
}: {
  error: Error & { digest?: string };
  unstable_retry: () => void;
}) {
  return (
    <ErrorFallback
      error={error}
      unstable_retry={unstable_retry}
      title="Could not load agents"
    />
  );
}
