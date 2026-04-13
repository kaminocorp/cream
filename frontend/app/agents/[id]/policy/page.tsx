import Link from "next/link";
import { PageHeader } from "@/components/shared/page-header";
import { PolicyEditor } from "@/components/policy/policy-editor";
import { AgentStatusBadge } from "@/components/shared/agent-status-badge";
import { getApiClient } from "@/lib/api";
import { requireAuth } from "@/lib/auth";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function PolicyPage({ params }: Props) {
  await requireAuth();
  const { id } = await params;
  const api = await getApiClient();
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
            <AgentStatusBadge status={agent.status} />
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
