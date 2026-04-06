import { PageHeader } from "@/components/shared/page-header";
import { DataTable, Column } from "@/components/shared/data-table";
import { StatusBadge } from "@/components/shared/status-badge";
import { AuditEntry } from "@/lib/types";
import { formatDate } from "@/lib/utils";

const columns: Column<AuditEntry>[] = [
  { key: "id",        header: "Entry",   cell: (r) => <span className="font-mono text-xs">{r.id.slice(0, 18)}…</span> },
  { key: "status",    header: "Status",  cell: (r) => <StatusBadge status={r.final_status} /> },
  { key: "decision",  header: "Decision",cell: (r) => r.policy_evaluation.final_decision },
  { key: "agent",     header: "Agent",   cell: (r) => <span className="font-mono text-xs">{r.agent_id}</span> },
  { key: "time",      header: "Time",    cell: (r) => formatDate(r.timestamp) },
];

export default function AuditPage() {
  // Phase 15: const entries = await getApiClient().queryAudit({ limit: 100 })
  const entries: AuditEntry[] = [];

  return (
    <div>
      <PageHeader title="Audit Log" description="Immutable record of all payment lifecycle events" />
      <div className="p-6">
        <DataTable
          columns={columns}
          data={entries}
          emptyTitle="Audit log is empty"
          emptyDescription="Every payment event — request, policy decision, routing, settlement — is recorded here."
        />
      </div>
    </div>
  );
}
