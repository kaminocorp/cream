"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ConditionTree } from "./condition-tree";
import { PolicyRule, PolicyAction } from "@/lib/types";
import { ChevronDown, ChevronRight } from "lucide-react";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const ACTION_STYLES: Record<PolicyAction, string> = {
  APPROVE: "bg-green-100 text-green-700",
  BLOCK: "bg-red-100 text-red-700",
  ESCALATE: "bg-yellow-100 text-yellow-700",
};

const RULE_TYPE_LABELS: Record<string, string> = {
  amount_cap: "Amount Cap",
  velocity_limit: "Velocity Limit",
  spend_rate: "Spend Rate",
  category_check: "Category Check",
  merchant_check: "Merchant Check",
  geographic: "Geographic",
  rail_restriction: "Rail Restriction",
  justification_quality: "Justification Quality",
  time_window: "Time Window",
  first_time_merchant: "First-Time Merchant",
  duplicate_detection: "Duplicate Detection",
  escalation_threshold: "Escalation Threshold",
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface RuleListProps {
  rules: PolicyRule[];
}

export function RuleList({ rules }: RuleListProps) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const sorted = [...rules].sort((a, b) => a.priority - b.priority);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  if (sorted.length === 0) {
    return (
      <p className="text-sm text-zinc-500">
        No policy rules configured for this profile.
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {sorted.map((rule) => {
        const isExpanded = expanded.has(rule.id);
        const typeLabel = rule.rule_type
          ? RULE_TYPE_LABELS[rule.rule_type] ?? rule.rule_type
          : "Custom";

        return (
          <Card
            key={rule.id}
            className={!rule.enabled ? "opacity-50" : undefined}
          >
            <CardHeader
              className="cursor-pointer pb-2"
              role="button"
              tabIndex={0}
              aria-expanded={isExpanded}
              onClick={() => toggle(rule.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  toggle(rule.id);
                }
              }}
            >
              <CardTitle className="flex items-center gap-2 text-sm">
                {isExpanded ? (
                  <ChevronDown className="h-4 w-4 text-zinc-400" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-zinc-400" />
                )}
                <span className="font-mono text-xs text-zinc-400">
                  #{rule.priority}
                </span>
                <span className="text-zinc-700">{typeLabel}</span>
                <Badge className={ACTION_STYLES[rule.action]}>{rule.action}</Badge>
                {!rule.enabled && (
                  <Badge className="bg-zinc-100 text-zinc-500">disabled</Badge>
                )}
                {rule.escalation && (
                  <span className="text-xs text-zinc-400">
                    → {rule.escalation.channel} ({rule.escalation.timeout_minutes}m,
                    on timeout: {rule.escalation.on_timeout})
                  </span>
                )}
              </CardTitle>
            </CardHeader>
            {isExpanded && (
              <CardContent className="pt-0">
                <div className="rounded-md border border-zinc-100 bg-zinc-50/50 p-3">
                  <ConditionTree condition={rule.condition} />
                </div>
                <div className="mt-2 font-mono text-[10px] text-zinc-400">
                  {rule.id}
                </div>
              </CardContent>
            )}
          </Card>
        );
      })}
    </div>
  );
}
