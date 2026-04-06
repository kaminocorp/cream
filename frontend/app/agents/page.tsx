import { PageHeader } from "@/components/shared/page-header";
import { EmptyState } from "@/components/shared/empty-state";
import { Users } from "lucide-react";

export default function AgentsPage() {
  // Phase 15: fetch agent list (requires a list-all-agents endpoint — not yet in Phase 8 API)
  return (
    <div>
      <PageHeader title="Agents" description="Registered agents and their policy profiles" />
      <div className="p-6">
        <EmptyState
          icon={Users}
          title="No agents configured"
          description="Agents registered with the control plane will appear here."
        />
      </div>
    </div>
  );
}
