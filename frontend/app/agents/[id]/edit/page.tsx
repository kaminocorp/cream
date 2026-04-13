import { PageHeader } from "@/components/shared/page-header";
import { AgentForm, ProfileOption } from "@/components/agents/agent-form";
import { getApiClient } from "@/lib/api";
import { requireAuth } from "@/lib/auth";

interface Props {
  params: Promise<{ id: string }>;
}

/**
 * Edit agent page. Fetches the agent's current state (via its policy
 * endpoint which returns the full agent + profile) and the agents list
 * (for the profile dropdown).
 */
export default async function EditAgentPage({ params }: Props) {
  await requireAuth();
  const { id } = await params;
  const api = await getApiClient();

  const [policy, agents] = await Promise.all([
    api.getAgentPolicy(id),
    api.listAgents(),
  ]);

  const { agent } = policy;

  // Derive unique profiles from existing agents.
  const profileMap = new Map<string, string>();
  for (const a of agents) {
    if (!profileMap.has(a.profile_id)) {
      profileMap.set(a.profile_id, a.profile_name);
    }
  }
  const profiles: ProfileOption[] = Array.from(profileMap, ([id, name]) => ({
    id,
    name,
  }));

  return (
    <div>
      <PageHeader
        title={`Edit ${agent.name}`}
        description={
          <span className="font-mono text-xs text-zinc-500">{agent.id}</span>
        }
      />
      <div className="p-6">
        <AgentForm
          mode="edit"
          profiles={profiles}
          initial={{
            id: agent.id,
            name: agent.name,
            status: agent.status,
            profileId: agent.profile_id,
          }}
        />
      </div>
    </div>
  );
}
