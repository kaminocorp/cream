import { PageHeader } from "@/components/shared/page-header";
import { PollingRefresh } from "@/components/shared/polling-refresh";
import { EscalationTable } from "@/components/escalations/escalation-table";
import { getApiClient } from "@/lib/api";

/**
 * Escalation queue. Server component fetches the pending-approval entries
 * and passes them to the `<EscalationTable>` client component which owns
 * the approve/reject buttons, optimistic UI, and error display.
 *
 * Polled every 5 seconds — escalation management is the highest-urgency
 * operator workflow in the dashboard.
 */
export default async function EscalationsPage() {
  const api = getApiClient();
  const pending = await api.queryAudit({
    status: "pending_approval",
    limit: 200,
  });

  return (
    <div>
      <PollingRefresh intervalMs={5_000} />
      <PageHeader
        title="Escalations"
        description={`${pending.length} payment${pending.length !== 1 ? "s" : ""} awaiting human approval`}
      />
      <div className="p-6">
        <EscalationTable entries={pending} />
      </div>
    </div>
  );
}
