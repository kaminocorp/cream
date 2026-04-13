import Link from "next/link";
import { PageHeader } from "@/components/shared/page-header";
import { DataTable, Column } from "@/components/shared/data-table";
import { EmptyState } from "@/components/shared/empty-state";
import { AgentStatusBadge } from "@/components/shared/agent-status-badge";
import { Button } from "@/components/ui/button";
import { getApiClient } from "@/lib/api";
import { AgentSummary } from "@/lib/types";
import { formatDate } from "@/lib/utils";
import { Plus, Users } from "lucide-react";

const columns: Column<AgentSummary>[] = [
  {
    key: "name",
    header: "Name",
    cell: (r) => (
      <Link
        href={`/agents/${r.id}`}
        className="font-medium text-zinc-900 hover:underline"
      >
        {r.name}
      </Link>
    ),
  },
  {
    key: "id",
    header: "ID",
    cell: (r) => <span className="font-mono text-xs text-zinc-500">{r.id}</span>,
  },
  {
    key: "profile",
    header: "Profile",
    cell: (r) => r.profile_name,
  },
  {
    key: "status",
    header: "Status",
    cell: (r) => <AgentStatusBadge status={r.status} />,
  },
  {
    key: "created",
    header: "Created",
    cell: (r) => formatDate(r.created_at),
  },
];

export default async function AgentsPage() {
  const api = await getApiClient();
  const agents = await api.listAgents();

  return (
    <div>
      <PageHeader
        title="Agents"
        description="Registered agents and their policy profiles"
      />
      <div className="p-6">
        <div className="mb-4 flex justify-end">
          <Link href="/agents/new">
            <Button size="sm">
              <Plus className="mr-1 h-3 w-3" data-icon="inline-start" />
              New Agent
            </Button>
          </Link>
        </div>
        {agents.length === 0 ? (
          <EmptyState
            icon={Users}
            title="No agents configured"
            description="Create your first agent to get started."
            action={
              <Link href="/agents/new">
                <Button size="sm">
                  <Plus className="mr-1 h-3 w-3" data-icon="inline-start" />
                  New Agent
                </Button>
              </Link>
            }
          />
        ) : (
          <DataTable columns={columns} data={agents} />
        )}
      </div>
    </div>
  );
}
