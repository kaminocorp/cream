import { Badge } from "@/components/ui/badge";
import {
  AuditEntry,
  Justification,
  PaymentRequest,
} from "@/lib/types";
import { formatAmount, formatDate } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-zinc-400">
        {title}
      </h4>
      {children}
    </div>
  );
}

function DL({ items }: { items: [string, React.ReactNode][] }) {
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
      {items
        .filter(([, v]) => v != null && v !== "" && v !== undefined)
        .map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-zinc-500">{label}</dt>
            <dd className="text-zinc-800 break-all">{value}</dd>
          </div>
        ))}
    </dl>
  );
}

function readAs<T>(blob: unknown): T | null {
  if (blob && typeof blob === "object") return blob as T;
  return null;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface AuditDetailPanelProps {
  entry: AuditEntry;
}

export function AuditDetailPanel({ entry }: AuditDetailPanelProps) {
  const req = readAs<PaymentRequest>(entry.request);
  const just = readAs<Justification>(entry.justification);
  const pe = entry.policy_evaluation;
  const rd = entry.routing_decision;
  const pr = entry.provider_response;
  const hr = entry.human_review;

  return (
    <div className="grid gap-4 rounded-md border border-zinc-100 bg-zinc-50/50 p-4 md:grid-cols-2">
      {/* Request */}
      {req && (
        <Section title="Request">
          <DL
            items={[
              ["Amount", req.amount && req.currency ? formatAmount(req.amount, String(req.currency)) : "—"],
              ["Currency", String(req.currency ?? "—")],
              ["Rail", req.preferred_rail],
              ["Recipient", req.recipient?.identifier ?? "—"],
              ["Recipient type", req.recipient?.type ?? "—"],
              ["Recipient name", req.recipient?.name ?? null],
              ["Recipient country", req.recipient?.country ?? null],
              ["Idempotency key", <code key="idem" className="font-mono text-xs">{req.idempotency_key}</code>],
            ]}
          />
        </Section>
      )}

      {/* Justification */}
      {just && (
        <Section title="Justification">
          <DL
            items={[
              ["Summary", just.summary],
              ["Category", typeof just.category === "string" ? just.category : just.category?.other ? `other: ${just.category.other}` : "—"],
              ["Task ID", just.task_id ?? null],
              ["Expected value", just.expected_value ?? null],
            ]}
          />
        </Section>
      )}

      {/* Policy evaluation */}
      <Section title="Policy Evaluation">
        <DL
          items={[
            ["Decision", <Badge key="dec">{pe.final_decision}</Badge>],
            ["Latency", `${pe.decision_latency_ms}ms`],
            ["Rules evaluated", pe.rules_evaluated.length > 0 ? pe.rules_evaluated.join(", ") : "none"],
            ["Matching rules", pe.matching_rules.length > 0 ? pe.matching_rules.join(", ") : "none"],
          ]}
        />
      </Section>

      {/* Routing decision */}
      {rd && (
        <Section title="Routing">
          <DL
            items={[
              ["Selected provider", rd.selected?.provider_id ?? "—"],
              ["Selected rail", rd.selected_rail ?? "—"],
              ["Reason", rd.reason ?? null],
              [
                "Candidates",
                rd.candidates.length > 0
                  ? rd.candidates.map((c) => `${c.provider_id} (score ${c.score})`).join(", ")
                  : "none",
              ],
            ]}
          />
        </Section>
      )}

      {/* Provider response */}
      {pr && (
        <Section title="Provider Response">
          <DL
            items={[
              ["Provider", pr.provider],
              ["Transaction ID", <code key="txid" className="font-mono text-xs">{pr.transaction_id}</code>],
              ["Status", pr.status],
              ["Settled", formatAmount(pr.amount_settled, String(pr.currency))],
              ["Latency", `${pr.latency_ms}ms`],
            ]}
          />
        </Section>
      )}

      {/* On-chain tx hash */}
      {entry.on_chain_tx_hash && (
        <Section title="On-Chain">
          <DL
            items={[
              ["TX hash", <code key="hash" className="font-mono text-xs">{entry.on_chain_tx_hash}</code>],
            ]}
          />
        </Section>
      )}

      {/* Human review */}
      {hr && (
        <Section title="Human Review">
          <DL
            items={[
              ["Reviewer", hr.reviewer_id],
              ["Decision", <Badge key="hrdec">{hr.decision}</Badge>],
              ["Reason", hr.reason ?? null],
              ["Decided at", formatDate(hr.decided_at)],
            ]}
          />
        </Section>
      )}
    </div>
  );
}
