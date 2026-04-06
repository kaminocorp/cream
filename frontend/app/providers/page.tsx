import { PageHeader } from "@/components/shared/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/shared/empty-state";
import { Activity } from "lucide-react";
import { ProviderHealth } from "@/lib/types";

function circuitBadge(state: ProviderHealth["circuit_state"]) {
  const map = {
    closed:    "bg-green-100 text-green-800",
    half_open: "bg-yellow-100 text-yellow-800",
    open:      "bg-red-100 text-red-800",
  } as const;
  return <Badge className={map[state]}>{state.replace("_", " ")}</Badge>;
}

export default async function ProvidersPage() {
  // Phase 15: const providers = await getApiClient().getProviderHealth()
  const providers: ProviderHealth[] = [];

  return (
    <div>
      <PageHeader title="Providers" description="Real-time health of connected payment providers" />
      <div className="p-6">
        {providers.length === 0 ? (
          <EmptyState
            icon={Activity}
            title="No provider data"
            description="Provider health metrics will appear here once providers are registered."
          />
        ) : (
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {providers.map((p) => (
              <Card key={p.provider_id}>
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-sm font-medium">{p.provider_id}</CardTitle>
                  {circuitBadge(p.circuit_state)}
                </CardHeader>
                <CardContent className="space-y-1 text-sm text-zinc-600">
                  <div className="flex justify-between">
                    <span>Error rate (5m)</span>
                    <span>{(p.error_rate_5m * 100).toFixed(1)}%</span>
                  </div>
                  <div className="flex justify-between">
                    <span>p50 latency</span>
                    <span>{p.p50_latency_ms}ms</span>
                  </div>
                  <div className="flex justify-between">
                    <span>p99 latency</span>
                    <span>{p.p99_latency_ms}ms</span>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
