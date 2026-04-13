import { Badge } from "@/components/ui/badge";
import { AgentStatus } from "@/lib/types";

const AGENT_STATUS_CLASSES: Record<AgentStatus, string> = {
  active: "bg-green-100 text-green-800",
  suspended: "bg-yellow-100 text-yellow-800",
  revoked: "bg-red-100 text-red-800",
};

export function AgentStatusBadge({ status }: { status: AgentStatus }) {
  return <Badge className={AGENT_STATUS_CLASSES[status]}>{status}</Badge>;
}
