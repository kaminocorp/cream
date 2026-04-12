"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  updatePolicy,
  UpdateProfileInput,
} from "@/app/agents/[id]/policy/actions";
import {
  AgentProfile,
  PaymentCategory,
  RailPreference,
} from "@/lib/types";
import { Loader2, X } from "lucide-react";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

const ALL_CATEGORIES: PaymentCategory[] = [
  "saas_subscription",
  "cloud_infrastructure",
  "api_credits",
  "travel",
  "procurement",
  "marketing",
  "legal",
  "other",
];

const ALL_RAILS: RailPreference[] = [
  "auto",
  "card",
  "ach",
  "swift",
  "local",
  "stablecoin",
];

const CATEGORY_LABELS: Record<PaymentCategory, string> = {
  saas_subscription: "SaaS",
  cloud_infrastructure: "Cloud",
  api_credits: "API Credits",
  travel: "Travel",
  procurement: "Procurement",
  marketing: "Marketing",
  legal: "Legal",
  other: "Other",
};

// ---------------------------------------------------------------------------
// Tag input helper
// ---------------------------------------------------------------------------

function TagToggle<T extends string>({
  options,
  selected,
  onToggle,
  labels,
}: {
  options: T[];
  selected: T[];
  onToggle: (value: T) => void;
  labels?: Record<T, string>;
}) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {options.map((opt) => {
        const active = selected.includes(opt);
        return (
          <button
            key={opt}
            type="button"
            onClick={() => onToggle(opt)}
            className={`rounded-md border px-2 py-0.5 text-xs transition-colors ${
              active
                ? "border-zinc-400 bg-zinc-100 text-zinc-800"
                : "border-zinc-200 text-zinc-400 hover:border-zinc-300 hover:text-zinc-600"
            }`}
          >
            {labels?.[opt] ?? opt}
          </button>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Geo restrictions — simple text tags
// ---------------------------------------------------------------------------

function GeoTags({
  values,
  onChange,
}: {
  values: string[];
  onChange: (v: string[]) => void;
}) {
  const [input, setInput] = useState("");

  const add = () => {
    const trimmed = input.trim().toUpperCase();
    if (trimmed && !values.includes(trimmed)) {
      onChange([...values, trimmed]);
    }
    setInput("");
  };

  const remove = (v: string) => {
    onChange(values.filter((x) => x !== v));
  };

  return (
    <div className="space-y-1.5">
      <div className="flex flex-wrap gap-1">
        {values.map((v) => (
          <Badge
            key={v}
            className="bg-zinc-100 text-zinc-700 gap-1"
          >
            {v}
            <button type="button" onClick={() => remove(v)} className="hover:text-red-600">
              <X className="h-3 w-3" />
            </button>
          </Badge>
        ))}
      </div>
      <div className="flex gap-1.5">
        <Input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add();
            }
          }}
          placeholder="e.g. SG"
          className="w-24"
        />
        <Button type="button" variant="outline" size="sm" onClick={add}>
          Add
        </Button>
      </div>
      <p className="text-[10px] text-zinc-400">
        ISO 3166-1 alpha-2 country codes. Empty = no restrictions.
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main form
// ---------------------------------------------------------------------------

interface ProfileFormProps {
  agentId: string;
  profile: AgentProfile;
}

export function ProfileForm({ agentId, profile }: ProfileFormProps) {
  const router = useRouter();
  const [isPending, startTransition] = useTransition();

  // Spending limits
  const [maxPerTx, setMaxPerTx] = useState(profile.max_per_transaction ?? "");
  const [maxDaily, setMaxDaily] = useState(profile.max_daily_spend ?? "");
  const [maxWeekly, setMaxWeekly] = useState(profile.max_weekly_spend ?? "");
  const [maxMonthly, setMaxMonthly] = useState(profile.max_monthly_spend ?? "");
  const [escalationThreshold, setEscalationThreshold] = useState(
    profile.escalation_threshold ?? "",
  );

  // Categories & rails
  const [categories, setCategories] = useState<PaymentCategory[]>(
    profile.allowed_categories ?? [],
  );
  const [rails, setRails] = useState<RailPreference[]>(
    profile.allowed_rails ?? [],
  );

  // Geographic restrictions
  const [geo, setGeo] = useState<string[]>(
    profile.geographic_restrictions ?? [],
  );

  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const toggleCategory = (cat: PaymentCategory) => {
    setCategories((prev) =>
      prev.includes(cat) ? prev.filter((c) => c !== cat) : [...prev, cat],
    );
  };

  const toggleRail = (rail: RailPreference) => {
    setRails((prev) =>
      prev.includes(rail) ? prev.filter((r) => r !== rail) : [...prev, rail],
    );
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSuccess(false);

    const input: UpdateProfileInput = {};

    // Only send changed fields.
    if (maxPerTx && maxPerTx !== profile.max_per_transaction) input.max_per_transaction = maxPerTx;
    if (maxDaily && maxDaily !== profile.max_daily_spend) input.max_daily_spend = maxDaily;
    if (maxWeekly && maxWeekly !== profile.max_weekly_spend) input.max_weekly_spend = maxWeekly;
    if (maxMonthly && maxMonthly !== profile.max_monthly_spend) input.max_monthly_spend = maxMonthly;
    if (escalationThreshold && escalationThreshold !== profile.escalation_threshold)
      input.escalation_threshold = escalationThreshold;

    // Always send categories/rails/geo if they differ.
    if (JSON.stringify(categories) !== JSON.stringify(profile.allowed_categories))
      input.allowed_categories = categories;
    if (JSON.stringify(rails) !== JSON.stringify(profile.allowed_rails))
      input.allowed_rails = rails;
    if (JSON.stringify(geo) !== JSON.stringify(profile.geographic_restrictions))
      input.geographic_restrictions = geo;

    if (Object.keys(input).length === 0) {
      setSuccess(true);
      return;
    }

    startTransition(async () => {
      const result = await updatePolicy(agentId, input);
      if (result.ok) {
        setSuccess(true);
        router.refresh();
      } else {
        setError(result.message);
      }
    });
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-5">
      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
          {error}
        </div>
      )}
      {success && (
        <div className="rounded-md border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-800">
          Profile updated successfully.
        </div>
      )}

      {/* Spending limits */}
      <fieldset className="space-y-3">
        <legend className="text-sm font-medium text-zinc-700">Spending Limits</legend>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <div className="space-y-1">
            <label htmlFor="max-per-tx" className="text-xs text-zinc-500">Per transaction</label>
            <Input
              id="max-per-tx"
              type="number"
              step="0.01"
              min="0.01"
              value={maxPerTx}
              onChange={(e) => setMaxPerTx(e.target.value)}
              placeholder="No limit"
            />
          </div>
          <div className="space-y-1">
            <label htmlFor="max-daily" className="text-xs text-zinc-500">Daily</label>
            <Input
              id="max-daily"
              type="number"
              step="0.01"
              min="0.01"
              value={maxDaily}
              onChange={(e) => setMaxDaily(e.target.value)}
              placeholder="No limit"
            />
          </div>
          <div className="space-y-1">
            <label htmlFor="max-weekly" className="text-xs text-zinc-500">Weekly</label>
            <Input
              id="max-weekly"
              type="number"
              step="0.01"
              min="0.01"
              value={maxWeekly}
              onChange={(e) => setMaxWeekly(e.target.value)}
              placeholder="No limit"
            />
          </div>
          <div className="space-y-1">
            <label htmlFor="max-monthly" className="text-xs text-zinc-500">Monthly</label>
            <Input
              id="max-monthly"
              type="number"
              step="0.01"
              min="0.01"
              value={maxMonthly}
              onChange={(e) => setMaxMonthly(e.target.value)}
              placeholder="No limit"
            />
          </div>
        </div>
        <div className="space-y-1">
          <label htmlFor="escalation-threshold" className="text-xs text-zinc-500">
            Escalation threshold (require human approval above this amount)
          </label>
          <Input
            id="escalation-threshold"
            type="number"
            step="0.01"
            min="0.01"
            value={escalationThreshold}
            onChange={(e) => setEscalationThreshold(e.target.value)}
            placeholder="No threshold"
            className="max-w-xs"
          />
        </div>
      </fieldset>

      {/* Allowed categories */}
      <fieldset className="space-y-2">
        <legend className="text-sm font-medium text-zinc-700">Allowed Categories</legend>
        <p className="text-xs text-zinc-400">
          Empty = all categories allowed. Selected = only these categories allowed.
        </p>
        <TagToggle
          options={ALL_CATEGORIES}
          selected={categories}
          onToggle={toggleCategory}
          labels={CATEGORY_LABELS}
        />
      </fieldset>

      {/* Allowed rails */}
      <fieldset className="space-y-2">
        <legend className="text-sm font-medium text-zinc-700">Allowed Rails</legend>
        <p className="text-xs text-zinc-400">
          Empty = all rails allowed. Selected = only these rails allowed.
        </p>
        <TagToggle
          options={ALL_RAILS}
          selected={rails}
          onToggle={toggleRail}
        />
      </fieldset>

      {/* Geographic restrictions */}
      <fieldset className="space-y-2">
        <legend className="text-sm font-medium text-zinc-700">Geographic Restrictions</legend>
        <GeoTags values={geo} onChange={setGeo} />
      </fieldset>

      {/* Submit */}
      <div className="flex gap-2 pt-2">
        <Button type="submit" disabled={isPending}>
          {isPending && <Loader2 className="mr-1 h-3 w-3 animate-spin" data-icon="inline-start" />}
          Save Profile
        </Button>
      </div>
    </form>
  );
}
