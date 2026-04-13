import { PageHeader } from "@/components/shared/page-header";
import { getApiClient } from "@/lib/api";
import { AgentSummary, PolicyTemplate } from "@/lib/types";
import { PoliciesClient } from "./policies-client";

export default async function PoliciesPage() {
  const api = await getApiClient();

  let agents: AgentSummary[] = [];
  let templates: PolicyTemplate[] = [];

  const [agentsResult, templatesResult] = await Promise.allSettled([
    api.listAgents(),
    api.listTemplates(),
  ]);

  if (agentsResult.status === "fulfilled") agents = agentsResult.value;
  if (templatesResult.status === "fulfilled") templates = templatesResult.value;

  return (
    <div>
      <PageHeader
        title="Policies"
        description="Policy rules and templates governing agent payments"
      />
      <div className="p-6">
        <PoliciesClient agents={agents} templates={templates} />
      </div>
    </div>
  );
}
