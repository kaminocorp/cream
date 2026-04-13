import { PageHeader } from "@/components/shared/page-header";
import { PollingRefresh } from "@/components/shared/polling-refresh";
import { DataTable, Column } from "@/components/shared/data-table";
import { StatusBadge } from "@/components/shared/status-badge";
import { getApiClient } from "@/lib/api";
import { requireAuth } from "@/lib/auth";
import { AuditEntry, Currency } from "@/lib/types";
import { formatAmount, formatDate } from "@/lib/utils";

/**
 * `AuditEntry.request` is typed as `unknown` because it is a full
 * `PaymentRequest` JSON blob persisted verbatim into the audit ledger.
 * We narrow it at the render boundary here rather than tightening the
 * type globally — the audit reader accepts any shape, and legacy entries
 * may not carry every field.
 */
interface PaymentRequestLike {
  amount?: string;
  currency?: Currency | string;
  agent_id?: string;
}

function readRequest(entry: AuditEntry): PaymentRequestLike {
  if (entry.request && typeof entry.request === "object") {
    return entry.request as PaymentRequestLike;
  }
  return {};
}

const columns: Column<AuditEntry>[] = [
  {
    key: "id",
    header: "Entry",
    cell: (r) => (
      <span className="font-mono text-xs">
        {r.payment_id ? r.payment_id.slice(0, 20) + "…" : r.id.slice(0, 20) + "…"}
      </span>
    ),
  },
  {
    key: "status",
    header: "Status",
    cell: (r) => <StatusBadge status={r.final_status} />,
  },
  {
    key: "amount",
    header: "Amount",
    cell: (r) => {
      const req = readRequest(r);
      return req.amount && req.currency
        ? formatAmount(req.amount, String(req.currency))
        : "—";
    },
  },
  {
    key: "agent",
    header: "Agent",
    cell: (r) => <span className="font-mono text-xs">{r.agent_id}</span>,
  },
  {
    key: "time",
    header: "Time",
    cell: (r) => formatDate(r.timestamp),
  },
];

/**
 * Transactions feed. Polled every 10 seconds via `PollingRefresh` so the
 * list stays fresh without WebSocket infrastructure (deferred post-Beta).
 * Operator-scoped via the shared `CREAM_API_KEY` → shows every agent's
 * payments, not just one.
 */
export default async function TransactionsPage() {
  await requireAuth();
  const api = await getApiClient();
  const entries = await api.queryAudit({ limit: 50 });

  return (
    <div>
      <PollingRefresh intervalMs={10_000} />
      <PageHeader
        title="Transactions"
        description={`${entries.length} most recent payment events — live updating every 10s`}
      />
      <div className="p-6">
        <DataTable
          columns={columns}
          data={entries}
          emptyTitle="No transactions yet"
          emptyDescription="Payments initiated by agents will appear here."
        />
      </div>
    </div>
  );
}
