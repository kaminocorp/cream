"use client";

import { useCallback } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { PaymentStatus, PaymentCategory, AgentId } from "@/lib/types";
import { X } from "lucide-react";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

const STATUS_OPTIONS: { value: PaymentStatus; label: string }[] = [
  { value: "pending", label: "Pending" },
  { value: "validating", label: "Validating" },
  { value: "pending_approval", label: "Pending Approval" },
  { value: "approved", label: "Approved" },
  { value: "submitted", label: "Submitted" },
  { value: "settled", label: "Settled" },
  { value: "failed", label: "Failed" },
  { value: "blocked", label: "Blocked" },
  { value: "rejected", label: "Rejected" },
  { value: "timed_out", label: "Timed Out" },
];

const CATEGORY_OPTIONS: { value: PaymentCategory; label: string }[] = [
  { value: "saas_subscription", label: "SaaS Subscription" },
  { value: "cloud_infrastructure", label: "Cloud Infrastructure" },
  { value: "api_credits", label: "API Credits" },
  { value: "travel", label: "Travel" },
  { value: "procurement", label: "Procurement" },
  { value: "marketing", label: "Marketing" },
  { value: "legal", label: "Legal" },
  { value: "other", label: "Other" },
];

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface AgentOption {
  id: AgentId;
  name: string;
}

interface AuditFilterBarProps {
  agents: AgentOption[];
}

// Sentinel used to represent "all" in a select — distinct from any real value.
const ALL = "__all__";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Convert an ISO 8601 UTC string back to the `datetime-local` input format
 * in the user's local timezone. Without this, stored UTC values shift by the
 * timezone offset on every page reload (e.g. 16:00 SGT → stored as 08:00Z →
 * displayed as 08:00 on next load).
 */
function isoToLocal(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "";
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AuditFilterBar({ agents }: AuditFilterBarProps) {
  const router = useRouter();
  const searchParams = useSearchParams();

  const current = {
    q: searchParams.get("q") ?? "",
    status: searchParams.get("status") ?? "",
    category: searchParams.get("category") ?? "",
    agent_id: searchParams.get("agent_id") ?? "",
    from: searchParams.get("from") ?? "",
    to: searchParams.get("to") ?? "",
    min_amount: searchParams.get("min_amount") ?? "",
    max_amount: searchParams.get("max_amount") ?? "",
  };

  const hasFilters = Object.values(current).some((v) => v !== "");

  const pushParams = useCallback(
    (updates: Record<string, string>) => {
      const params = new URLSearchParams(searchParams.toString());
      for (const [key, val] of Object.entries(updates)) {
        if (val) {
          params.set(key, val);
        } else {
          params.delete(key);
        }
      }
      // Reset offset when filters change.
      params.delete("offset");
      router.push(`/audit?${params.toString()}`);
    },
    [router, searchParams],
  );

  const clearAll = () => {
    router.push("/audit");
  };

  return (
    <div className="space-y-3">
      {/* Row 1: Search + Status + Category + Agent */}
      <div className="flex flex-wrap items-end gap-2">
        {/* Free-text search */}
        <div className="min-w-[200px] flex-1 space-y-1">
          <label htmlFor="audit-search" className="text-xs font-medium text-zinc-500">
            Search justifications
          </label>
          <Input
            id="audit-search"
            key={current.q}
            placeholder="Search..."
            defaultValue={current.q}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                pushParams({ q: (e.target as HTMLInputElement).value });
              }
            }}
            onBlur={(e) => {
              if (e.target.value !== current.q) {
                pushParams({ q: e.target.value });
              }
            }}
          />
        </div>

        {/* Status */}
        <div className="space-y-1">
          <label className="text-xs font-medium text-zinc-500">Status</label>
          <Select
            value={current.status || ALL}
            onValueChange={(v) => pushParams({ status: v === ALL ? "" : (v ?? "") })}
          >
            <SelectTrigger className="w-[150px]">
              <SelectValue placeholder="All" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>All</SelectItem>
              {STATUS_OPTIONS.map((o) => (
                <SelectItem key={o.value} value={o.value}>
                  {o.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Category */}
        <div className="space-y-1">
          <label className="text-xs font-medium text-zinc-500">Category</label>
          <Select
            value={current.category || ALL}
            onValueChange={(v) => pushParams({ category: v === ALL ? "" : (v ?? "") })}
          >
            <SelectTrigger className="w-[170px]">
              <SelectValue placeholder="All" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>All</SelectItem>
              {CATEGORY_OPTIONS.map((o) => (
                <SelectItem key={o.value} value={o.value}>
                  {o.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Agent */}
        {agents.length > 0 && (
          <div className="space-y-1">
            <label className="text-xs font-medium text-zinc-500">Agent</label>
            <Select
              value={current.agent_id || ALL}
              onValueChange={(v) => pushParams({ agent_id: v === ALL ? "" : (v ?? "") })}
            >
              <SelectTrigger className="w-[170px]">
                <SelectValue placeholder="All agents" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={ALL}>All agents</SelectItem>
                {agents.map((a) => (
                  <SelectItem key={a.id} value={a.id}>
                    {a.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}
      </div>

      {/* Row 2: Date range + Amount range + Clear */}
      <div className="flex flex-wrap items-end gap-2">
        <div className="space-y-1">
          <label htmlFor="audit-from" className="text-xs font-medium text-zinc-500">
            From
          </label>
          <Input
            id="audit-from"
            type="datetime-local"
            className="w-[190px]"
            key={`from-${current.from}`}
            defaultValue={current.from ? isoToLocal(current.from) : ""}
            onChange={(e) => {
              const val = e.target.value;
              pushParams({ from: val ? new Date(val).toISOString() : "" });
            }}
          />
        </div>

        <div className="space-y-1">
          <label htmlFor="audit-to" className="text-xs font-medium text-zinc-500">
            To
          </label>
          <Input
            id="audit-to"
            type="datetime-local"
            className="w-[190px]"
            key={`to-${current.to}`}
            defaultValue={current.to ? isoToLocal(current.to) : ""}
            onChange={(e) => {
              const val = e.target.value;
              pushParams({ to: val ? new Date(val).toISOString() : "" });
            }}
          />
        </div>

        <div className="space-y-1">
          <label htmlFor="audit-min" className="text-xs font-medium text-zinc-500">
            Min amount
          </label>
          <Input
            id="audit-min"
            type="number"
            step="0.01"
            min="0"
            className="w-[120px]"
            placeholder="0.00"
            key={`min-${current.min_amount}`}
            defaultValue={current.min_amount}
            onBlur={(e) => pushParams({ min_amount: e.target.value })}
            onKeyDown={(e) => {
              if (e.key === "Enter") pushParams({ min_amount: (e.target as HTMLInputElement).value });
            }}
          />
        </div>

        <div className="space-y-1">
          <label htmlFor="audit-max" className="text-xs font-medium text-zinc-500">
            Max amount
          </label>
          <Input
            id="audit-max"
            type="number"
            step="0.01"
            min="0"
            className="w-[120px]"
            placeholder="∞"
            key={`max-${current.max_amount}`}
            defaultValue={current.max_amount}
            onBlur={(e) => pushParams({ max_amount: e.target.value })}
            onKeyDown={(e) => {
              if (e.key === "Enter") pushParams({ max_amount: (e.target as HTMLInputElement).value });
            }}
          />
        </div>

        {hasFilters && (
          <Button variant="ghost" size="sm" onClick={clearAll}>
            <X className="mr-1 h-3 w-3" />
            Clear filters
          </Button>
        )}
      </div>
    </div>
  );
}
