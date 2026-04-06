import { PageHeader } from "@/components/shared/page-header";
import { EmptyState } from "@/components/shared/empty-state";
import { ShieldCheck } from "lucide-react";

export default function PoliciesPage() {
  // Phase 15: const { rules } = await getApiClient().getAgentPolicy(agentId)
  return (
    <div>
      <PageHeader title="Policies" description="Policy rules governing agent payments" />
      <div className="p-6">
        <EmptyState
          icon={ShieldCheck}
          title="No policy rules"
          description="Policy rules controlling spend limits, categories, and escalation will appear here."
        />
      </div>
    </div>
  );
}
