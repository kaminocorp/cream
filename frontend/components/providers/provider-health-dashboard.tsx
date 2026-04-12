"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/shared/empty-state";
import { HealthChart, ChartDataPoint } from "./health-chart";
import { fetchProviderHealth } from "@/app/providers/actions";
import { ProviderHealth, CircuitState } from "@/lib/types";
import { Activity } from "lucide-react";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const POLL_INTERVAL_MS = 10_000;
const MAX_SAMPLES = 60; // 10 min of history at 10s intervals

// ---------------------------------------------------------------------------
// Ring buffer — one per provider
// ---------------------------------------------------------------------------

interface Snapshot {
  t: number; // seconds since mount
  health: ProviderHealth;
}

type HistoryMap = Map<string, Snapshot[]>;

function pushSnapshot(
  history: HistoryMap,
  providers: ProviderHealth[],
  elapsed: number,
): HistoryMap {
  const next = new Map(history);
  for (const p of providers) {
    const buf = next.get(p.provider_id) ?? [];
    const updated = [...buf, { t: elapsed, health: p }];
    // Ring buffer: keep only the last MAX_SAMPLES entries.
    next.set(
      p.provider_id,
      updated.length > MAX_SAMPLES ? updated.slice(-MAX_SAMPLES) : updated,
    );
  }
  return next;
}

// ---------------------------------------------------------------------------
// Chart data builders
// ---------------------------------------------------------------------------

function formatElapsed(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return m > 0 ? `${m}m${s > 0 ? ` ${s}s` : ""}` : `${s}s`;
}

function errorRateData(snapshots: Snapshot[]): ChartDataPoint[] {
  return snapshots.map((s) => ({
    t: s.t,
    label: formatElapsed(s.t),
    error_rate: parseFloat((s.health.error_rate_5m * 100).toFixed(2)),
  }));
}

function latencyData(snapshots: Snapshot[]): ChartDataPoint[] {
  return snapshots.map((s) => ({
    t: s.t,
    label: formatElapsed(s.t),
    p50: s.health.p50_latency_ms,
    p99: s.health.p99_latency_ms,
  }));
}

// ---------------------------------------------------------------------------
// Circuit state badge
// ---------------------------------------------------------------------------

function circuitBadge(state: CircuitState) {
  const styles: Record<CircuitState, string> = {
    closed: "bg-green-100 text-green-800",
    half_open: "bg-yellow-100 text-yellow-800",
    open: "bg-red-100 text-red-800",
  };
  return <Badge className={styles[state]}>{state.replace("_", " ")}</Badge>;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface Props {
  initial: ProviderHealth[];
}

export function ProviderHealthDashboard({ initial }: Props) {
  const mountTime = useRef(0);
  const [history, setHistory] = useState<HistoryMap>(() =>
    pushSnapshot(new Map(), initial, 0),
  );
  const [latest, setLatest] = useState<ProviderHealth[]>(initial);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  // Capture mount time in effect to satisfy React purity rules.
  useEffect(() => {
    mountTime.current = Date.now();
  }, []);

  const poll = useCallback(async () => {
    try {
      const data = await fetchProviderHealth();
      const elapsed = mountTime.current
        ? Math.round((Date.now() - mountTime.current) / 1000)
        : 0;
      setHistory((prev) => pushSnapshot(prev, data, elapsed));
      setLatest(data);
      setLastUpdated(new Date());
    } catch {
      // Silently skip failed polls — the chart just doesn't get a new data point.
    }
  }, []);

  useEffect(() => {
    // Pause when tab is hidden to avoid wasting resources.
    let timer: ReturnType<typeof setInterval> | null = null;

    const start = () => {
      if (!timer) timer = setInterval(poll, POLL_INTERVAL_MS);
    };
    const stop = () => {
      if (timer) {
        clearInterval(timer);
        timer = null;
      }
    };

    const onVisibility = () => {
      if (document.hidden) {
        stop();
      } else {
        poll(); // Immediate refresh on tab focus.
        start();
      }
    };

    start();
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [poll]);

  if (latest.length === 0) {
    return (
      <EmptyState
        icon={Activity}
        title="No provider data"
        description="Provider health metrics will appear here once providers are registered."
      />
    );
  }

  return (
    <div className="space-y-4">
      <p className="text-xs text-zinc-400">
        Last updated: {lastUpdated ? lastUpdated.toLocaleTimeString() : "now"} · Polling every{" "}
        {POLL_INTERVAL_MS / 1000}s · Ring buffer:{" "}
        {Math.max(...Array.from(history.values()).map((s) => s.length))} samples
      </p>

      <div className="grid gap-4 lg:grid-cols-2">
        {latest.map((p) => {
          const snapshots = history.get(p.provider_id) ?? [];

          return (
            <Card
              key={p.provider_id}
              className={p.is_healthy ? undefined : "border-red-200"}
            >
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium">
                  {p.provider_id}
                </CardTitle>
                <div className="flex items-center gap-2">
                  <span
                    className={`inline-block h-2 w-2 rounded-full ${
                      p.is_healthy ? "bg-green-500" : "bg-red-500"
                    }`}
                    title={p.is_healthy ? "Healthy" : "Unhealthy"}
                  />
                  {circuitBadge(p.circuit_state)}
                </div>
              </CardHeader>

              <CardContent className="space-y-4">
                {/* Current metrics summary */}
                <div className="grid grid-cols-4 gap-2 text-center text-xs">
                  <div>
                    <div className="font-semibold">
                      {(p.error_rate_5m * 100).toFixed(1)}%
                    </div>
                    <div className="text-zinc-400">Error rate</div>
                  </div>
                  <div>
                    <div className="font-semibold">{p.p50_latency_ms}ms</div>
                    <div className="text-zinc-400">p50</div>
                  </div>
                  <div>
                    <div className="font-semibold">{p.p99_latency_ms}ms</div>
                    <div className="text-zinc-400">p99</div>
                  </div>
                  <div>
                    <div className="font-semibold">{p.is_healthy ? "✓" : "✗"}</div>
                    <div className="text-zinc-400">Health</div>
                  </div>
                </div>

                {/* Error rate chart */}
                <div>
                  <h4 className="mb-1 text-xs font-medium text-zinc-500">
                    Error Rate (%)
                  </h4>
                  <HealthChart
                    data={errorRateData(snapshots)}
                    lines={[
                      { key: "error_rate", name: "Error %", color: "#ef4444" },
                    ]}
                    yLabel="%"
                    yDomain={[0, 100]}
                    yFormat={(v) => `${v}`}
                  />
                </div>

                {/* Latency chart */}
                <div>
                  <h4 className="mb-1 text-xs font-medium text-zinc-500">
                    Latency (ms)
                  </h4>
                  <HealthChart
                    data={latencyData(snapshots)}
                    lines={[
                      { key: "p50", name: "p50", color: "#3b82f6" },
                      { key: "p99", name: "p99", color: "#f59e0b" },
                    ]}
                    yLabel="ms"
                  />
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
