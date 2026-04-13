import { PageHeader } from "@/components/shared/page-header";
import { AgentForm, ProfileOption } from "@/components/agents/agent-form";
import { getApiClient } from "@/lib/api";
import { requireAuth } from "@/lib/auth";

/**
 * Create agent page. Fetches the agent list to derive available profiles
 * for the dropdown — no dedicated list-profiles endpoint exists yet, so
 * we extract unique (id, name) pairs from the agents table.
 */
export default async function NewAgentPage() {
  await requireAuth();
  const api = await getApiClient();
  const agents = await api.listAgents();

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
        title="New Agent"
        description="Register a new agent and receive its one-time API key."
      />
      <div className="p-6">
        <AgentForm mode="create" profiles={profiles} />
      </div>
    </div>
  );
}
