"use server";

import { revalidatePath } from "next/cache";
import { getApiClient } from "@/lib/api";
import { ApiError, AgentStatus } from "@/lib/types";

// ---------------------------------------------------------------------------
// Result types — plain objects for server/client serialization boundary
// ---------------------------------------------------------------------------

export type ActionResult =
  | { ok: true }
  | { ok: false; message: string };

export type CreateAgentResult =
  | { ok: true; agentId: string; apiKey: string }
  | { ok: false; message: string };

export type RotateKeyResult =
  | { ok: true; apiKey: string }
  | { ok: false; message: string };

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function createAgent(
  name: string,
  profileId: string,
): Promise<CreateAgentResult> {
  try {
    const api = getApiClient();
    const res = await api.createAgent({ name, profile_id: profileId });

    revalidatePath("/agents");

    return { ok: true, agentId: res.agent.id, apiKey: res.api_key };
  } catch (err) {
    const message =
      err instanceof ApiError
        ? err.message
        : "An unexpected error occurred while creating the agent";
    return { ok: false, message };
  }
}

export async function updateAgent(
  agentId: string,
  update: { name?: string; status?: AgentStatus; profile_id?: string },
): Promise<ActionResult> {
  try {
    const api = getApiClient();
    await api.updateAgent(agentId, update);

    revalidatePath("/agents");
    revalidatePath(`/agents/${agentId}`);

    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError
        ? err.message
        : "An unexpected error occurred while updating the agent";
    return { ok: false, message };
  }
}

export async function rotateAgentKey(
  agentId: string,
): Promise<RotateKeyResult> {
  try {
    const api = getApiClient();
    const res = await api.rotateAgentKey(agentId);

    revalidatePath(`/agents/${agentId}`);

    return { ok: true, apiKey: res.api_key };
  } catch (err) {
    const message =
      err instanceof ApiError
        ? err.message
        : "An unexpected error occurred while rotating the key";
    return { ok: false, message };
  }
}
