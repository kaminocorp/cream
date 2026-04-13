"use server";

import { revalidatePath } from "next/cache";
import { getApiClient } from "@/lib/api";
import { ApiError } from "@/lib/types";

export type ActionResult =
  | { ok: true }
  | { ok: false; message: string };

export async function applyTemplate(
  templateId: string,
  agentId: string,
): Promise<ActionResult> {
  try {
    const api = await getApiClient();
    await api.applyTemplate(templateId, agentId);
    revalidatePath("/policies");
    revalidatePath(`/agents/${agentId}/policy`);
    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError ? err.message : "Failed to apply template";
    return { ok: false, message };
  }
}
