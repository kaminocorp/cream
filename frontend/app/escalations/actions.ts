"use server";

import { revalidatePath } from "next/cache";
import { getApiClient } from "@/lib/api";
import { ApiError } from "@/lib/types";

/**
 * Result shape returned to the client component. A plain object — not an
 * Error instance — so it can cross the server/client serialization boundary
 * cleanly. React Server Functions must return serializable values.
 */
export type ActionResult =
  | { ok: true }
  | { ok: false; message: string };

/**
 * Approve an escalated payment. Called by the `<EscalationTable>` client
 * component via `startTransition`.
 *
 * The `reviewer_id` is a free-text label identifying which operator made
 * the decision — it's stored verbatim in the append-only audit ledger.
 * Phase 16-A will populate this automatically from the authenticated
 * operator's identity; until then the dashboard hardcodes a placeholder.
 */
export async function approveEscalation(
  paymentId: string,
  reviewerId: string = "dashboard-operator",
): Promise<ActionResult> {
  try {
    const api = getApiClient();
    await api.approvePayment(paymentId, reviewerId);

    // Revalidate the escalations page so the next render reflects the
    // removal. The client-side optimistic UI handles the instant visual
    // feedback; this ensures the server-rendered HTML is also up to date
    // when the user navigates away and back.
    revalidatePath("/escalations");
    revalidatePath("/");

    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError
        ? err.message
        : "An unexpected error occurred while approving";
    return { ok: false, message };
  }
}

/**
 * Reject an escalated payment. Same contract as `approveEscalation`.
 */
export async function rejectEscalation(
  paymentId: string,
  reviewerId: string = "dashboard-operator",
  reason?: string,
): Promise<ActionResult> {
  try {
    const api = getApiClient();
    await api.rejectPayment(paymentId, reviewerId, reason);

    revalidatePath("/escalations");
    revalidatePath("/");

    return { ok: true };
  } catch (err) {
    const message =
      err instanceof ApiError
        ? err.message
        : "An unexpected error occurred while rejecting";
    return { ok: false, message };
  }
}
