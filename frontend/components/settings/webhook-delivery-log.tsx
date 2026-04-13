"use client";

import { WebhookDelivery } from "@/lib/types";
import { formatDate } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";

interface WebhookDeliveryLogProps {
  deliveries: WebhookDelivery[];
}

function deliveryStatusColor(status: string): string {
  switch (status) {
    case "delivered":
      return "bg-green-100 text-green-800";
    case "failed":
      return "bg-yellow-100 text-yellow-800";
    case "exhausted":
      return "bg-red-100 text-red-800";
    default:
      return "bg-zinc-100 text-zinc-700";
  }
}

export function WebhookDeliveryLog({ deliveries }: WebhookDeliveryLogProps) {
  if (deliveries.length === 0) {
    return (
      <p className="text-sm text-zinc-400">No deliveries yet.</p>
    );
  }

  return (
    <div className="space-y-2">
      {deliveries.map((d) => (
        <div
          key={d.id}
          className="flex items-center justify-between rounded-md border px-3 py-2 text-sm"
        >
          <div className="flex items-center gap-3">
            <Badge className={deliveryStatusColor(d.status)}>{d.status}</Badge>
            <span className="font-mono text-xs text-zinc-500">{d.event_type}</span>
          </div>
          <div className="flex items-center gap-3 text-xs text-zinc-400">
            <span>
              Attempt {d.attempt}/{d.max_attempts}
            </span>
            {d.http_status && <span>HTTP {d.http_status}</span>}
            <span>{formatDate(d.created_at)}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
