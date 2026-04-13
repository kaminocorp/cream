"use server";

import { revalidatePath } from "next/cache";
import { getApiClient } from "@/lib/api";
import { ApiError } from "@/lib/types";

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

export type ActionResult =
  | { ok: true }
  | { ok: false; message: string };

// ---------------------------------------------------------------------------
// Webhook actions
// ---------------------------------------------------------------------------

export async function registerWebhook(
  url: string,
  secret: string,
  events: string[],
): Promise<ActionResult> {
  if (!url.trim()) return { ok: false, message: "URL is required" };
  if (secret.length < 16) return { ok: false, message: "Secret must be at least 16 characters" };

  try {
    const api = await getApiClient();
    await api.registerWebhook({ url: url.trim(), secret, events });
    revalidatePath("/settings");
    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError ? err.message : "Failed to register webhook";
    return { ok: false, message };
  }
}

export async function deleteWebhook(id: string): Promise<ActionResult> {
  try {
    const api = await getApiClient();
    await api.deleteWebhook(id);
    revalidatePath("/settings");
    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError ? err.message : "Failed to delete webhook";
    return { ok: false, message };
  }
}

export async function testWebhook(id: string): Promise<ActionResult> {
  try {
    const api = await getApiClient();
    await api.testWebhook(id);
    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError ? err.message : "Failed to send test event";
    return { ok: false, message };
  }
}

// ---------------------------------------------------------------------------
// Provider key actions
// ---------------------------------------------------------------------------

export async function saveProviderKey(
  providerName: string,
  apiKey: string,
): Promise<ActionResult> {
  if (!apiKey.trim()) return { ok: false, message: "API key is required" };

  try {
    const api = await getApiClient();
    await api.saveProviderKey(providerName, apiKey);
    revalidatePath("/settings");
    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError ? err.message : "Failed to save provider key";
    return { ok: false, message };
  }
}
