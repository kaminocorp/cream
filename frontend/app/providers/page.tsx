import { PageHeader } from "@/components/shared/page-header";
import { ProviderHealthDashboard } from "@/components/providers/provider-health-dashboard";
import { getApiClient } from "@/lib/api";
import { requireAuth } from "@/lib/auth";

/**
 * Provider health page. The server component fetches the initial snapshot
 * and passes it to the client dashboard which handles its own 10-second
 * polling interval and accumulates snapshots in a ring buffer for the
 * time-series charts.
 *
 * No `<PollingRefresh>` here — the client component owns its own polling
 * because it needs to accumulate history, not replace it.
 */
export default async function ProvidersPage() {
  await requireAuth();
  const api = await getApiClient();
  const providers = await api.getProviderHealth();

  return (
    <div>
      <PageHeader
        title="Providers"
        description="Real-time health of connected payment providers"
      />
      <div className="p-6">
        <ProviderHealthDashboard initial={providers} />
      </div>
    </div>
  );
}
