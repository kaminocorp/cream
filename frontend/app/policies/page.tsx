import Link from "next/link";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState } from "@/components/shared/empty-state";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { getApiClient } from "@/lib/api";
import { ShieldCheck } from "lucide-react";

/**
 * Policies index. Per-agent rules live at `/agents/{id}/policy`; there is
 * no aggregate "all rules" endpoint. For 15.2 we surface each agent as an
 * entry point to its policy. The visual rule editor lives at
 * `/agents/{id}/policy` (shipped in Phase 15.7).
 *
 * The distinction between "policies" and "agents" here is that this page
 * is rule-focused (what governs the spend?) while `/agents` is
 * identity-focused (who is spending?). Same backing data, different
 * navigation intent.
 */
export default async function PoliciesPage() {
  const api = getApiClient();
  const agents = await api.listAgents();

  if (agents.length === 0) {
    return (
      <div>
        <PageHeader
          title="Policies"
          description="Policy rules governing agent payments"
        />
        <div className="p-6">
          <EmptyState
            icon={ShieldCheck}
            title="No policies to show"
            description="Create an agent first — policies are attached to agent profiles."
          />
        </div>
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Policies"
        description="Select an agent to view its active policy rules"
      />
      <div className="p-6">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {agents.map((a) => (
            <Link key={a.id} href={`/agents/${a.id}/policy`}>
              <Card className="transition-colors hover:border-zinc-300">
                <CardHeader className="pb-2">
                  <CardTitle className="flex items-center justify-between text-sm">
                    <span>{a.name}</span>
                    <Badge>{a.status}</Badge>
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-xs text-zinc-500">
                  <div>profile: {a.profile_name}</div>
                  <div className="mt-1 font-mono">{a.id}</div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
        <p className="mt-6 text-xs text-zinc-400">
          Select an agent to edit its profile settings and view policy rules.
        </p>
      </div>
    </div>
  );
}
