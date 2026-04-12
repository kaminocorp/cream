import Link from "next/link";
import { PageHeader } from "@/components/shared/page-header";
import { PolicyEditor } from "@/components/policy/policy-editor";
import { Badge } from "@/components/ui/badge";
import { getApiClient } from "@/lib/api";
import { AgentStatus } from "@/lib/types";

interface Props {
  params: Promise<{ id: string }>;
}

function statusBadge(status: AgentStatus) {
  const classes: Record<AgentStatus, string> = {
    active: "bg-green-100 text-green-800",
    suspended: "bg-yellow-100 text-yellow-800",
    revoked: "bg-red-100 text-red-800",
  };
  return <Badge className={classes[status]}>{status}</Badge>;
}

export default async function PolicyPage({ params }: Props) {
  const { id } = await params;
  const api = getApiClient();
  const { agent, profile, rules } = await api.getAgentPolicy(id);

  return (
    <div>
      <PageHeader
        title={`Policy: ${agent.name}`}
        description={
          <span className="flex items-center gap-2">
            <Link
              href={`/agents/${agent.id}`}
              className="text-xs text-zinc-500 hover:underline"
            >
              ← back to agent
            </Link>
            <span>·</span>
            {statusBadge(agent.status)}
            <span>·</span>
            <span className="text-xs text-zinc-500">
              profile: {profile.name} (v{profile.version})
            </span>
          </span>
        }
      />
      <div className="p-6">
        <PolicyEditor agentId={agent.id} profile={profile} rules={rules} />
      </div>
    </div>
  );
}
