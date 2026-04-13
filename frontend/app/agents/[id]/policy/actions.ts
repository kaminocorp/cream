"use server";

import { revalidatePath } from "next/cache";
import { getApiClient } from "@/lib/api";
import { ApiError, PaymentCategory, RailPreference } from "@/lib/types";

export type ActionResult =
  | { ok: true }
  | { ok: false; message: string };

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export interface UpdateProfileInput {
  max_per_transaction?: string | null;
  max_daily_spend?: string | null;
  max_weekly_spend?: string | null;
  max_monthly_spend?: string | null;
  allowed_categories?: PaymentCategory[];
  allowed_rails?: RailPreference[];
  geographic_restrictions?: string[];
  escalation_threshold?: string | null;
}

export async function updatePolicy(
  agentId: string,
  input: UpdateProfileInput,
): Promise<ActionResult> {
  if (!UUID_RE.test(agentId)) {
    return { ok: false, message: "Invalid agent ID format" };
  }
  try {
    const api = getApiClient();
    await api.updateAgentPolicy(agentId, input);

    revalidatePath(`/agents/${agentId}`);
    revalidatePath(`/agents/${agentId}/policy`);
    revalidatePath("/policies");

    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError
        ? err.message
        : "An unexpected error occurred while updating the policy";
    return { ok: false, message };
  }
}
