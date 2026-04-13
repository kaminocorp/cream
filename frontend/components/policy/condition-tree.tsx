import { Badge } from "@/components/ui/badge";
import { PolicyCondition } from "@/lib/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Readable names for comparison operators. */
const OP_LABELS: Record<string, string> = {
  equals: "=",
  not_equals: "≠",
  greater_than: ">",
  less_than: "<",
  greater_than_or_equal: "≥",
  less_than_or_equal: "≤",
  in: "in",
  not_in: "not in",
  contains: "contains",
  matches: "matches",
  // Serde variants (PascalCase) — handle both casings
  Equals: "=",
  NotEquals: "≠",
  GreaterThan: ">",
  LessThan: "<",
  GreaterThanOrEqual: "≥",
  LessThanOrEqual: "≤",
  In: "in",
  NotIn: "not in",
  Contains: "contains",
  Matches: "matches",
};

function formatValue(value: unknown): string {
  if (value === null || value === undefined) return "null";
  if (Array.isArray(value)) {
    if (value.length <= 5) return `[${value.join(", ")}]`;
    return `[${value.slice(0, 3).join(", ")}, … +${value.length - 3}]`;
  }
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

// ---------------------------------------------------------------------------
// Detect condition variant
// ---------------------------------------------------------------------------

type ConditionVariant =
  | { type: "All"; children: PolicyCondition[] }
  | { type: "Any"; children: PolicyCondition[] }
  | { type: "Not"; child: PolicyCondition }
  | { type: "FieldCheck"; field: string; op: string; value: unknown };

function classify(cond: PolicyCondition): ConditionVariant {
  if ("all" in cond) return { type: "All", children: cond.all };
  if ("any" in cond) return { type: "Any", children: cond.any };
  if ("not" in cond) return { type: "Not", child: cond.not };
  if ("field_check" in cond) {
    const fc = cond.field_check;
    return { type: "FieldCheck", field: fc.field, op: fc.op, value: fc.value };
  }
  // Fallback for unexpected / future condition shapes — render visibly
  // so operators notice rather than silently masking unknown variants.
  return { type: "Unknown" as "FieldCheck", field: "unknown", op: "?", value: cond };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface ConditionTreeProps {
  condition: PolicyCondition;
  depth?: number;
}

/**
 * Recursive condition tree renderer. Each node is either a logical combinator
 * (All/Any/Not) rendered as a labeled group, or a FieldCheck leaf rendered
 * inline.
 *
 * Indentation increases with depth via `ml-4` on child groups.
 * Max visual depth mirrors the backend's 32-level limit but in practice
 * rules rarely exceed 3-4 levels.
 */
const MAX_DEPTH = 32;

export function ConditionTree({ condition, depth = 0 }: ConditionTreeProps) {
  if (depth >= MAX_DEPTH) {
    return (
      <span className="text-xs italic text-zinc-400">
        … (max nesting depth reached)
      </span>
    );
  }

  const variant = classify(condition);

  if (variant.type === "FieldCheck") {
    return (
      <div className="flex flex-wrap items-center gap-1.5 text-sm">
        <code className="rounded bg-zinc-100 px-1.5 py-0.5 font-mono text-xs text-zinc-700">
          {variant.field}
        </code>
        <span className="font-medium text-zinc-500">
          {OP_LABELS[variant.op] ?? variant.op}
        </span>
        <code className="rounded bg-blue-50 px-1.5 py-0.5 font-mono text-xs text-blue-700">
          {formatValue(variant.value)}
        </code>
      </div>
    );
  }

  if (variant.type === "Not") {
    return (
      <div className={depth > 0 ? "ml-4" : undefined}>
        <div className="flex items-center gap-1.5">
          <Badge className="bg-red-100 text-red-700 text-[10px]">NOT</Badge>
        </div>
        <div className="ml-4 mt-1 border-l-2 border-red-200 pl-3">
          <ConditionTree condition={variant.child} depth={depth + 1} />
        </div>
      </div>
    );
  }

  // All or Any
  const isAll = variant.type === "All";
  const label = isAll ? "ALL" : "ANY";
  const borderColor = isAll ? "border-indigo-200" : "border-amber-200";
  const badgeColor = isAll
    ? "bg-indigo-100 text-indigo-700"
    : "bg-amber-100 text-amber-700";

  return (
    <div className={depth > 0 ? "ml-4" : undefined}>
      <div className="flex items-center gap-1.5">
        <Badge className={`${badgeColor} text-[10px]`}>{label}</Badge>
        <span className="text-xs text-zinc-400">
          ({variant.children.length} condition{variant.children.length !== 1 ? "s" : ""})
        </span>
      </div>
      <div className={`ml-4 mt-1 space-y-2 border-l-2 ${borderColor} pl-3`}>
        {variant.children.map((child, i) => (
          <ConditionTree key={i} condition={child} depth={depth + 1} />
        ))}
      </div>
    </div>
  );
}
