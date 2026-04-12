"use client";

import { useOptimistic, useTransition, useState } from "react";
import { useRouter } from "next/navigation";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/shared/empty-state";
import { AuditEntry } from "@/lib/types";
import { formatAmount, relativeTime, formatDate } from "@/lib/utils";
import {
  approveEscalation,
  rejectEscalation,
  ActionResult,
} from "@/app/escalations/actions";
import { AlertTriangle, Check, X, Loader2 } from "lucide-react";

// ---------------------------------------------------------------------------
// Type helpers for narrowing the unknown JSON blobs
// ---------------------------------------------------------------------------

interface RequestLike {
  amount?: string;
  currency?: string;
}

interface JustificationLike {
  summary?: string;
  category?: string;
}

function readRequest(entry: AuditEntry): RequestLike {
  if (entry.request && typeof entry.request === "object") {
    return entry.request as RequestLike;
  }
  return {};
}

function readJustification(entry: AuditEntry): JustificationLike {
  if (entry.justification && typeof entry.justification === "object") {
    return entry.justification as JustificationLike;
  }
  return {};
}

// ---------------------------------------------------------------------------
// Optimistic state
// ---------------------------------------------------------------------------

type OptimisticAction =
  | { type: "remove"; paymentId: string }
  | { type: "restore"; paymentId: string; entry: AuditEntry };

function optimisticReducer(
  state: AuditEntry[],
  action: OptimisticAction,
): AuditEntry[] {
  switch (action.type) {
    case "remove":
      return state.filter((e) => e.payment_id !== action.paymentId);
    case "restore":
      return [...state, action.entry];
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface EscalationTableProps {
  entries: AuditEntry[];
}

/**
 * Interactive escalation queue. Client component that receives pending
 * `AuditEntry[]` from the server page and renders approve/reject buttons
 * wired to Server Actions.
 *
 * **Optimistic UI.** When the operator clicks approve or reject, the row
 * vanishes instantly via `useOptimistic`. If the server action fails, the
 * row re-appears with an inline error message. This prevents the 200ms+
 * round-trip from making the UI feel sluggish on every decision.
 *
 * **Error handling.** Errors are shown inline below the table as transient
 * banners (auto-dismiss after 5 seconds). Full toast infrastructure lands
 * in Phase 15.8 polish.
 *
 * **Countdown.** The plan noted a per-row timeout countdown, but the
 * `AuditEntry` doesn't carry the escalation deadline today (the timeout is
 * on the `PolicyRule.escalation.timeout_minutes`, which isn't joined into
 * the audit response). For 15.3 we show "waiting since" via
 * `relativeTime(timestamp)`. A proper countdown is a 15.8 polish item
 * that requires either (a) enriching the audit entry with a computed
 * `escalation_deadline_utc` or (b) a client-side join with the policy
 * rules.
 */
export function EscalationTable({ entries }: EscalationTableProps) {
  const router = useRouter();
  const [isPending, startTransition] = useTransition();
  const [optimisticEntries, dispatchOptimistic] = useOptimistic(
    entries,
    optimisticReducer,
  );
  const [error, setError] = useState<string | null>(null);

  const handleAction = (
    paymentId: string,
    entry: AuditEntry,
    action: "approve" | "reject",
  ) => {
    setError(null);

    startTransition(async () => {
      // Optimistically remove the row.
      dispatchOptimistic({ type: "remove", paymentId });

      let result: ActionResult;
      if (action === "approve") {
        result = await approveEscalation(paymentId);
      } else {
        result = await rejectEscalation(paymentId);
      }

      if (!result.ok) {
        // Restore the row on failure — the optimistic removal is unwound
        // when the transition settles, but we also need to surface the
        // error to the operator.
        setError(`Failed to ${action}: ${result.message}`);
      }

      // Refresh the server data whether success or failure so the list
      // reflects the authoritative state.
      router.refresh();
    });
  };

  // Auto-dismiss errors after 5 seconds.
  if (error) {
    setTimeout(() => setError(null), 5_000);
  }

  if (optimisticEntries.length === 0) {
    return (
      <EmptyState
        icon={AlertTriangle}
        title="No pending escalations"
        description="Payments flagged for human review will appear here with approve and reject controls."
      />
    );
  }

  return (
    <div>
      {error && (
        <div className="mb-4 rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
          {error}
        </div>
      )}

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Payment</TableHead>
            <TableHead>Amount</TableHead>
            <TableHead>Agent</TableHead>
            <TableHead>Justification</TableHead>
            <TableHead>Waiting</TableHead>
            <TableHead className="text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {optimisticEntries.map((entry) => {
            const req = readRequest(entry);
            const just = readJustification(entry);
            const paymentId = entry.payment_id ?? entry.id;

            return (
              <TableRow key={entry.id}>
                <TableCell>
                  <span className="font-mono text-xs">{paymentId}</span>
                </TableCell>
                <TableCell>
                  {req.amount && req.currency
                    ? formatAmount(req.amount, String(req.currency))
                    : "—"}
                </TableCell>
                <TableCell>
                  <span className="font-mono text-xs">{entry.agent_id}</span>
                </TableCell>
                <TableCell>
                  <span className="line-clamp-1 max-w-[240px] text-sm text-zinc-700">
                    {just.summary ?? "—"}
                  </span>
                </TableCell>
                <TableCell>
                  <span
                    className="text-xs text-zinc-500"
                    title={formatDate(entry.timestamp)}
                  >
                    {relativeTime(entry.timestamp)}
                  </span>
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex justify-end gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      className="border-green-200 text-green-700 hover:bg-green-50"
                      disabled={isPending}
                      onClick={() => handleAction(paymentId, entry, "approve")}
                    >
                      {isPending ? (
                        <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                      ) : (
                        <Check className="mr-1 h-3 w-3" />
                      )}
                      Approve
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="border-red-200 text-red-700 hover:bg-red-50"
                      disabled={isPending}
                      onClick={() => handleAction(paymentId, entry, "reject")}
                    >
                      {isPending ? (
                        <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                      ) : (
                        <X className="mr-1 h-3 w-3" />
                      )}
                      Reject
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>

      {isPending && (
        <div className="mt-2 flex items-center gap-2 text-xs text-zinc-400">
          <Loader2 className="h-3 w-3 animate-spin" />
          Processing...
        </div>
      )}
    </div>
  );
}
