import { PageHeader } from "@/components/shared/page-header";
import { EmptyState } from "@/components/shared/empty-state";
import { AlertTriangle } from "lucide-react";
// Phase 15: import EscalationTable (client component with approve/reject handlers)

export default function EscalationsPage() {
  // Phase 15: const pending = await getApiClient().queryAudit({ status: "pending_approval" })
  const pendingCount: number = 0;

  return (
    <div>
      <PageHeader
        title="Escalations"
        description={`${pendingCount} payment${pendingCount !== 1 ? "s" : ""} awaiting human approval`}
      />
      <div className="p-6">
        <EmptyState
          icon={AlertTriangle}
          title="No pending escalations"
          description="Payments flagged for human review will appear here with approve and reject controls."
        />
      </div>
    </div>
  );
}
