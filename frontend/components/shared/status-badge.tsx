import { Badge } from "@/components/ui/badge";
import { cn, statusColor } from "@/lib/utils";
import { PaymentStatus } from "@/lib/types";

export function StatusBadge({ status }: { status: PaymentStatus }) {
  return (
    <Badge className={cn("text-xs font-medium", statusColor(status))}>
      {status.replace(/_/g, " ")}
    </Badge>
  );
}
