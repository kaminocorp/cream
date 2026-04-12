"use server";

import { getApiClient } from "@/lib/api";
import { ProviderHealth } from "@/lib/types";

/**
 * Fetch the current provider health snapshot. Called by the client-side
 * polling loop in `<ProviderHealthDashboard>` to accumulate time-series
 * data in a ring buffer.
 */
export async function fetchProviderHealth(): Promise<ProviderHealth[]> {
  const api = getApiClient();
  return api.getProviderHealth();
}
