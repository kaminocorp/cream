"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

/**
 * Invisible client component that calls `router.refresh()` on an interval.
 * Embed in any server-rendered page that should show live-updating data
 * without a full WebSocket/SSE stack.
 *
 * `router.refresh()` re-runs the server component tree, fetches fresh data,
 * and diffs the RSC payload into the DOM — no full page reload, no client
 * state loss. This is the cheapest possible "live feed" for Beta.
 *
 * Strategy rationale: deferred real-time streaming to post-Beta per the
 * Phase 15 plan (`docs/executing/phase-15-implementation-plan.md` §1,
 * Non-goals). When a page wants faster refresh than 10s, override with
 * a smaller `intervalMs`; when it wants none, omit this component.
 *
 * The interval pauses automatically when the tab is hidden (via
 * `document.visibilitychange`) so background tabs don't hammer the API.
 */
export function PollingRefresh({ intervalMs = 10_000 }: { intervalMs?: number }) {
  const router = useRouter();

  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | null = null;

    const start = () => {
      if (timer !== null) return;
      timer = setInterval(() => {
        // Only refresh when the tab is visible; otherwise let the next
        // visibilitychange event re-arm the interval.
        if (document.visibilityState === "visible") {
          router.refresh();
        }
      }, intervalMs);
    };

    const stop = () => {
      if (timer !== null) {
        clearInterval(timer);
        timer = null;
      }
    };

    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        start();
        // Also refresh immediately on tab focus — the data is likely stale.
        router.refresh();
      } else {
        stop();
      }
    };

    start();
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      stop();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [intervalMs, router]);

  return null;
}
