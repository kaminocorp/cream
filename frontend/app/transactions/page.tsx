import { PageHeader } from "@/components/shared/page-header";
import { DataTable, Column } from "@/components/shared/data-table";
import { StatusBadge } from "@/components/shared/status-badge";
import { PaymentResponse } from "@/lib/types";
import { formatAmount, formatDate } from "@/lib/utils";

const columns: Column<PaymentResponse>[] = [
  { key: "id",       header: "ID",       cell: (r) => <span className="font-mono text-xs">{r.id.slice(0, 20)}…</span> },
  { key: "status",   header: "Status",   cell: (r) => <StatusBadge status={r.status} /> },
  { key: "amount",   header: "Amount",   cell: (r) => formatAmount(r.request.amount, r.request.currency) },
  { key: "agent",    header: "Agent",    cell: (r) => <span className="font-mono text-xs">{r.request.agent_id}</span> },
  { key: "created",  header: "Created",  cell: (r) => formatDate(r.created_at) },
];

export default function TransactionsPage() {
  // Phase 15: replace with getApiClient().queryAudit({ limit: 50 })
  const payments: PaymentResponse[] = [];

  return (
    <div>
      <PageHeader title="Transactions" description="All payment requests" />
      <div className="p-6">
        <DataTable
          columns={columns}
          data={payments}
          emptyTitle="No transactions yet"
          emptyDescription="Payments initiated by agents will appear here."
        />
      </div>
    </div>
  );
}
