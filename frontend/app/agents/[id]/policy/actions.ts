"use server";

import { revalidatePath } from "next/cache";
import { getApiClient } from "@/lib/api";
import { ApiError, PaymentCategory, RailPreference } from "@/lib/types";

export type ActionResult =
  | { ok: true }
  | { ok: false; message: string };

export interface UpdateProfileInput {
  max_per_transaction?: string;
  max_daily_spend?: string;
  max_weekly_spend?: string;
  max_monthly_spend?: string;
  allowed_categories?: PaymentCategory[];
  allowed_rails?: RailPreference[];
  geographic_restrictions?: string[];
  escalation_threshold?: string;
}

export async function updatePolicy(
  agentId: string,
  input: UpdateProfileInput,
): Promise<ActionResult> {
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
