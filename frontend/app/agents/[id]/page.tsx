import Link from "next/link";
import { PageHeader } from "@/components/shared/page-header";
import { DataTable, Column } from "@/components/shared/data-table";
import { StatusBadge } from "@/components/shared/status-badge";
import { EmptyState } from "@/components/shared/empty-state";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { AgentStatusBadge } from "@/components/shared/agent-status-badge";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { getApiClient } from "@/lib/api";
import { AuditEntry, Currency, PolicyRule } from "@/lib/types";
import { formatAmount, formatDate } from "@/lib/utils";
import { RotateKeyDialog } from "@/components/agents/rotate-key-dialog";
import { BarChart2, Pencil } from "lucide-react";

interface Props {
  params: Promise<{ id: string }>;
}

const txColumns: Column<AuditEntry>[] = [
  {
    key: "id",
    header: "Payment",
    cell: (r) => (
      <span className="font-mono text-xs">
        {r.payment_id ?? r.id}
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
      const req = r.request as { amount?: string; currency?: Currency | string } | null;
      return req?.amount && req?.currency
        ? formatAmount(req.amount, String(req.currency))
        : "—";
    },
  },
  {
    key: "time",
    header: "Time",
    cell: (r) => formatDate(r.timestamp),
  },
];

/**
 * Agent detail page. Uses the extended `AgentPolicyResponse` from Phase
 * 15.2 (now carries `agent`, `profile`, `rules`) so the header can show
 * name + status without a second round-trip. Recent transactions are
 * fetched via `queryAudit({ agent_id, limit })` — the new `agent_id`
 * filter on the operator path.
 *
 * Full policy editor available at /agents/{id}/policy. Here we show a read-only
 * summary of the rules in the profile.
 */
export default async function AgentDetailPage({ params }: Props) {
  const { id } = await params;
  const api = await getApiClient();

  const [policy, recentTx] = await Promise.all([
    api.getAgentPolicy(id),
    api.queryAudit({ agent_id: id, limit: 20 }),
  ]);

  const { agent, profile, rules } = policy;

  const limitCards = [
    { label: "Per Transaction", value: profile.max_per_transaction },
    { label: "Daily", value: profile.max_daily_spend },
    { label: "Weekly", value: profile.max_weekly_spend },
    { label: "Monthly", value: profile.max_monthly_spend },
  ];

  return (
    <div>
      <PageHeader
        title={agent.name}
        description={
          <span className="flex items-center gap-2">
            <span className="font-mono text-xs text-zinc-500">{agent.id}</span>
            <span>·</span>
            <AgentStatusBadge status={agent.status} />
            <span>·</span>
            <span>profile: {profile.name}</span>
          </span>
        }
      />
      <div className="space-y-6 p-6">
        <div className="flex gap-2">
          <Link href={`/agents/${agent.id}/edit`}>
            <Button variant="outline" size="sm">
              <Pencil className="mr-1 h-3 w-3" data-icon="inline-start" />
              Edit
            </Button>
          </Link>
          <RotateKeyDialog agentId={agent.id} agentName={agent.name} />
        </div>

        <section>
          <h2 className="mb-2 text-sm font-medium text-zinc-600">Spending limits</h2>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            {limitCards.map(({ label, value }) => (
              <Card key={label}>
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm text-zinc-500">{label}</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="text-lg font-semibold">
                    {value ?? "—"}
                  </div>
                  <p className="mt-1 text-xs text-zinc-400">
                    {value ? "Limit" : "Not set"}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>

        <section>
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-sm font-medium text-zinc-600">Policy rules</h2>
            <Link
              href={`/agents/${agent.id}/policy`}
              className="text-xs text-zinc-500 hover:underline"
            >
              edit policy →
            </Link>
          </div>
          <Card>
            <CardContent className="pt-4">
              {rules.length === 0 ? (
                <p className="text-sm text-zinc-500">No rules configured.</p>
              ) : (
                <ul className="space-y-2">
                  {rules.map((r: PolicyRule) => (
                    <li
                      key={r.id}
                      className="flex items-center justify-between rounded-md border border-zinc-100 p-3 text-sm"
                    >
                      <div>
                        <span className="font-mono text-xs text-zinc-500">
                          priority {r.priority}
                        </span>
                        <span className="mx-2">·</span>
                        <span className="text-zinc-700">
                          {r.rule_type ?? "custom"}
                        </span>
                      </div>
                      <Badge>{r.action}</Badge>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        </section>

        <section>
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-sm font-medium text-zinc-600">Recent transactions</h2>
            <Link
              href={`/audit?agent_id=${id}`}
              className="text-xs text-zinc-500 hover:underline"
            >
              view all →
            </Link>
          </div>
          {recentTx.length === 0 ? (
            <EmptyState
              icon={BarChart2}
              title="No recent transactions"
              description="Recent payments for this agent will appear here."
            />
          ) : (
            <DataTable columns={txColumns} data={recentTx} />
          )}
        </section>
      </div>
    </div>
  );
}
