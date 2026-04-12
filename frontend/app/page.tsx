import { PageHeader } from "@/components/shared/page-header";
import { PollingRefresh } from "@/components/shared/polling-refresh";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { getApiClient } from "@/lib/api";
import { Users, AlertTriangle, Activity, FileText } from "lucide-react";

/**
 * Overview — the dashboard's landing page. Shows four honest metrics
 * computed from the data that's actually available:
 *
 * 1. Active Agents — count of agents with `status === "active"` from the
 *    operator-scoped `/v1/agents` list.
 * 2. Pending Escalations — count of audit entries with
 *    `final_status === "pending_approval"`. This is the operator's most
 *    urgent queue.
 * 3. Providers Online — count of entries in `/v1/providers/health` with
 *    `is_healthy === true`. Circuit breaker state is surfaced on the
 *    providers page.
 * 4. Recent Events (24h) — count of audit entries with `timestamp >= 24h
 *    ago`. Bounded by the query's 1000-row hard cap.
 *
 * All four queries run in `Promise.all` so the page's time-to-render is
 * bounded by the slowest single call, not the sum.
 */
/**
 * Compute the "24 hours ago" ISO timestamp outside the component body so
 * Next 16's React purity lint (`react-hooks/purity`) doesn't flag the
 * direct `Date.now()` call during render. Server components are run
 * per-request, so this helper is effectively recomputed on each call.
 */
function isoTimestampHoursAgo(hours: number): string {
  return new Date(Date.now() - hours * 60 * 60 * 1000).toISOString();
}

export default async function DashboardPage() {
  const api = getApiClient();
  const oneDayAgo = isoTimestampHoursAgo(24);

  // Fetch everything in parallel.
  const [agents, pendingEscalations, providerHealth, recentEvents] = await Promise.all([
    api.listAgents(),
    api.queryAudit({ status: "pending_approval", limit: 1000 }),
    api.getProviderHealth(),
    api.queryAudit({ from: oneDayAgo, limit: 1000 }),
  ]);

  const activeAgents = agents.filter((a) => a.status === "active").length;
  const healthyProviders = providerHealth.filter((p) => p.is_healthy).length;

  const summaryCards = [
    {
      title: "Active Agents",
      value: activeAgents.toString(),
      icon: Users,
      description: `${agents.length} total registered`,
    },
    {
      title: "Pending Escalations",
      value: pendingEscalations.length.toString(),
      icon: AlertTriangle,
      description: pendingEscalations.length > 0 ? "Awaiting human approval" : "Queue empty",
      urgent: pendingEscalations.length > 0,
    },
    {
      title: "Providers Online",
      value: `${healthyProviders}/${providerHealth.length}`,
      icon: Activity,
      description:
        providerHealth.length === 0 ? "No providers registered" : "Healthy / total",
    },
    {
      title: "Events (24h)",
      value: recentEvents.length.toString(),
      icon: FileText,
      description: "Audit ledger entries",
    },
  ];

  return (
    <div>
      <PollingRefresh intervalMs={10_000} />
      <PageHeader title="Overview" description="Payment control plane summary" />
      <div className="p-6">
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          {summaryCards.map((card) => (
            <Card
              key={card.title}
              className={card.urgent ? "border-yellow-400" : undefined}
            >
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-zinc-500">
                  {card.title}
                </CardTitle>
                <card.icon className="h-4 w-4 text-zinc-400" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{card.value}</div>
                <p className="mt-0.5 text-xs text-zinc-400">{card.description}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
