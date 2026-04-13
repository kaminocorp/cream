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
 * Resolve the operator reviewer identity for audit attribution.
 *
 * Phase 16-A will replace this with authenticated operator identity from
 * session tokens. Until then, we use `OPERATOR_REVIEWER_NAME` env var
 * (falls back to "dashboard-operator" if unset). This is a deliberate
 * improvement over a compile-time constant — operators can set a
 * meaningful label (e.g. "ops-team@acme") without code changes.
 */
function getReviewerId(): string {
  return process.env.OPERATOR_REVIEWER_NAME || "dashboard-operator";
}

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Approve an escalated payment. Called by the `<EscalationTable>` client
 * component via `startTransition`.
 *
 * The `reviewer_id` is a free-text label identifying which operator made
 * the decision — it's stored verbatim in the append-only audit ledger.
 * Phase 16-A will replace `getReviewerId()` with the authenticated
 * operator's identity from session tokens.
 */
export async function approveEscalation(
  paymentId: string,
  // TODO(Phase 16-A): replace getReviewerId() with authenticated operator identity
  reviewerId: string = getReviewerId(),
): Promise<ActionResult> {
  if (!UUID_RE.test(paymentId)) {
    return { ok: false, message: "Invalid payment ID format" };
  }
  try {
    const api = await getApiClient();
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
  // TODO(Phase 16-A): replace getReviewerId() with authenticated operator identity
  reviewerId: string = getReviewerId(),
  reason?: string,
): Promise<ActionResult> {
  if (!UUID_RE.test(paymentId)) {
    return { ok: false, message: "Invalid payment ID format" };
  }
  try {
    const api = await getApiClient();
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
