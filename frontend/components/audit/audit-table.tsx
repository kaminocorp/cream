"use client";

import { Fragment, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/shared/status-badge";
import { EmptyState } from "@/components/shared/empty-state";
import { AuditDetailPanel } from "./audit-detail-panel";
import { AuditEntry, Currency } from "@/lib/types";
import { formatAmount, formatDate } from "@/lib/utils";
import { ChevronDown, ChevronRight, FileText } from "lucide-react";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface RequestLike {
  amount?: string;
  currency?: Currency | string;
}

interface JustificationLike {
  summary?: string;
}

function readRequest(entry: AuditEntry): RequestLike {
  if (entry.request && typeof entry.request === "object") return entry.request as RequestLike;
  return {};
}

function readJustification(entry: AuditEntry): JustificationLike {
  if (entry.justification && typeof entry.justification === "object")
    return entry.justification as JustificationLike;
  return {};
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const PAGE_SIZE = 50;
const COL_COUNT = 8;

interface AuditTableProps {
  entries: AuditEntry[];
  hasMore: boolean;
}

export function AuditTable({ entries, hasMore }: AuditTableProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const offset = parseInt(searchParams.get("offset") ?? "0", 10);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const navigate = (newOffset: number) => {
    const params = new URLSearchParams(searchParams.toString());
    if (newOffset > 0) {
      params.set("offset", newOffset.toString());
    } else {
      params.delete("offset");
    }
    router.push(`/audit?${params.toString()}`);
  };

  if (entries.length === 0) {
    return (
      <EmptyState
        icon={FileText}
        title="No matching entries"
        description="Try adjusting your filters, or clear them to see all entries."
      />
    );
  }

  return (
    <div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-8" />
            <TableHead>Payment</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Decision</TableHead>
            <TableHead>Amount</TableHead>
            <TableHead>Agent</TableHead>
            <TableHead>Justification</TableHead>
            <TableHead>Time</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {entries.map((entry) => {
            const isExpanded = expanded.has(entry.id);
            const req = readRequest(entry);
            const just = readJustification(entry);

            return (
              <Fragment key={entry.id}>
                <TableRow
                  className="cursor-pointer hover:bg-zinc-50"
                  role="button"
                  tabIndex={0}
                  aria-expanded={isExpanded}
                  onClick={() => toggle(entry.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      toggle(entry.id);
                    }
                  }}
                >
                  <TableCell>
                    {isExpanded ? (
                      <ChevronDown className="h-4 w-4 text-zinc-400" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-zinc-400" />
                    )}
                  </TableCell>
                  <TableCell>
                    <span className="font-mono text-xs">
                      {(entry.payment_id ?? entry.id).slice(0, 18)}…
                    </span>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={entry.final_status} />
                  </TableCell>
                  <TableCell>
                    <span className="text-xs">{entry.policy_evaluation.final_decision}</span>
                  </TableCell>
                  <TableCell>
                    {req.amount && req.currency
                      ? formatAmount(req.amount, String(req.currency))
                      : "—"}
                  </TableCell>
                  <TableCell>
                    <span className="font-mono text-xs">
                      {entry.agent_id.slice(0, 18)}…
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="line-clamp-1 max-w-[200px] text-xs text-zinc-600">
                      {just.summary ?? "—"}
                    </span>
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-zinc-500">
                      {formatDate(entry.timestamp)}
                    </span>
                  </TableCell>
                </TableRow>
                {isExpanded && (
                  <TableRow className="hover:bg-transparent">
                    <TableCell colSpan={COL_COUNT} className="p-2">
                      <AuditDetailPanel entry={entry} />
                    </TableCell>
                  </TableRow>
                )}
              </Fragment>
            );
          })}
        </TableBody>
      </Table>

      {/* Pagination */}
      <div className="mt-4 flex items-center justify-between">
        <span className="text-xs text-zinc-500">
          Showing {offset + 1}–{offset + entries.length}
          {hasMore ? "+" : ""}
        </span>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={offset === 0}
            onClick={() => navigate(Math.max(0, offset - PAGE_SIZE))}
          >
            Previous
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={!hasMore}
            onClick={() => navigate(offset + PAGE_SIZE)}
          >
            Next
          </Button>
        </div>
      </div>
    </div>
  );
}

export { PAGE_SIZE };
