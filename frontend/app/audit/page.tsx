import { Suspense } from "react";
import { PageHeader } from "@/components/shared/page-header";
import { AuditFilterBar, AgentOption } from "@/components/audit/audit-filter-bar";
import { AuditTable, PAGE_SIZE } from "@/components/audit/audit-table";
import { getApiClient } from "@/lib/api";
import { AuditQueryFilters, PaymentStatus } from "@/lib/types";

interface Props {
  searchParams: Promise<Record<string, string | undefined>>;
}

/**
 * Audit log page — the operator's primary investigation tool.
 *
 * All filter state lives in URL search params so audit views are
 * shareable and bookmarkable. The filter bar (client component) reads
 * and writes params; this server page reads them and fetches accordingly.
 */
export default async function AuditPage({ searchParams }: Props) {
  const params = await searchParams;
  const api = await getApiClient();

  // Build query filters from URL search params.
  const filters: AuditQueryFilters = {
    limit: PAGE_SIZE,
  };
  if (params.q)          filters.q = params.q;
  if (params.status)     filters.status = params.status as PaymentStatus;
  if (params.category)   filters.category = params.category;
  if (params.agent_id)   filters.agent_id = params.agent_id;
  if (params.from)       filters.from = params.from;
  if (params.to)         filters.to = params.to;
  if (params.min_amount) filters.min_amount = params.min_amount;
  if (params.max_amount) filters.max_amount = params.max_amount;
  if (params.offset)     filters.offset = parseInt(params.offset, 10);

  // Fetch audit entries and agent list in parallel.
  const [entries, agents] = await Promise.all([
    api.queryAudit(filters),
    api.listAgents(),
  ]);

  // Derive agent options for the filter dropdown.
  const agentOptions: AgentOption[] = agents.map((a) => ({
    id: a.id,
    name: a.name,
  }));

  // If we got exactly PAGE_SIZE results, there may be more.
  const hasMore = entries.length === PAGE_SIZE;

  const activeFilterCount = [
    params.q, params.status, params.category, params.agent_id,
    params.from, params.to, params.min_amount, params.max_amount,
  ].filter(Boolean).length;

  return (
    <div>
      <PageHeader
        title="Audit Log"
        description={
          activeFilterCount > 0
            ? `${entries.length} entries matching ${activeFilterCount} filter${activeFilterCount !== 1 ? "s" : ""}`
            : `${entries.length} most recent entries across all agents`
        }
      />
      <div className="space-y-4 p-6">
        <Suspense>
          <AuditFilterBar agents={agentOptions} />
        </Suspense>
        <Suspense>
          <AuditTable entries={entries} hasMore={hasMore} />
        </Suspense>
      </div>
    </div>
  );
}
