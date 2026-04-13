"use client";

import { useState, useTransition, useRef, useEffect } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ApiKeyDisplay } from "./api-key-display";
import {
  createAgent,
  updateAgent,
  CreateAgentResult,
  ActionResult,
} from "@/app/agents/actions";
import { AgentStatus } from "@/lib/types";
import { Loader2 } from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ProfileOption {
  id: string;
  name: string;
}

interface AgentFormProps {
  mode: "create" | "edit";
  profiles: ProfileOption[];
  /** Pre-populated values for edit mode. */
  initial?: {
    id: string;
    name: string;
    status: AgentStatus;
    profileId: string;
  };
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

function validate(
  mode: "create" | "edit",
  name: string,
  profileId: string,
): string | null {
  if (!name.trim()) return "Agent name is required.";
  if (name.trim().length > 255) return "Name must be 255 characters or fewer.";
  if (mode === "create" && !profileId) return "A profile must be selected.";
  return null;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AgentForm({ mode, profiles, initial }: AgentFormProps) {
  const router = useRouter();
  const [isPending, startTransition] = useTransition();

  const [name, setName] = useState(initial?.name ?? "");
  const [profileId, setProfileId] = useState(initial?.profileId ?? "");
  const [status, setStatus] = useState<AgentStatus>(initial?.status ?? "active");
  const [error, setError] = useState<string | null>(null);
  const [fieldError, setFieldError] = useState<string | null>(null);

  // Post-create: one-time API key display.
  // The ref is a safety net — if the component re-renders due to an error
  // boundary above us, React state is lost but refs survive until unmount.
  const createdKeyRef = useRef<string | null>(null);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [createdAgentId, setCreatedAgentId] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setFieldError(null);

    const validationError = validate(mode, name, profileId);
    if (validationError) {
      setFieldError(validationError);
      return;
    }

    startTransition(async () => {
      if (mode === "create") {
        const result: CreateAgentResult = await createAgent(
          name.trim(),
          profileId,
        );
        if (result.ok) {
          createdKeyRef.current = result.apiKey;
          setCreatedKey(result.apiKey);
          setCreatedAgentId(result.agentId);
        } else {
          setError(result.message);
        }
      } else {
        const update: { name?: string; status?: AgentStatus; profile_id?: string } = {};
        if (name.trim() !== initial?.name) update.name = name.trim();
        if (status !== initial?.status) update.status = status;
        if (profileId !== initial?.profileId) update.profile_id = profileId;

        if (Object.keys(update).length === 0) {
          router.push(`/agents/${initial!.id}`);
          return;
        }

        const result: ActionResult = await updateAgent(initial!.id, update);
        if (result.ok) {
          router.push(`/agents/${initial!.id}`);
        } else {
          setError(result.message);
        }
      }
    });
  };

  // After successful create, show the API key instead of the form.
  // The ref is a safety net — if the component re-renders and React state is
  // lost (e.g. error boundary recovery), the effect below restores it from
  // the ref so the operator never loses sight of their one-time API key.
  useEffect(() => {
    if (!createdKey && createdKeyRef.current) {
      setCreatedKey(createdKeyRef.current);
    }
  }, [createdKey]);

  const displayKey = createdKey;
  if (displayKey) {
    return (
      <div className="mx-auto max-w-md space-y-4">
        <h2 className="text-base font-medium">Agent created</h2>
        <p className="text-sm text-zinc-600">
          Save the API key below. You will not be able to see it again.
        </p>
        <ApiKeyDisplay
          apiKey={displayKey}
          onAcknowledge={() => {
            createdKeyRef.current = null;
            router.push(`/agents/${createdAgentId}`);
          }}
        />
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} className="mx-auto max-w-md space-y-5">
      {error && (
        <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
          {error}
        </div>
      )}

      {fieldError && (
        <div className="rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800">
          {fieldError}
        </div>
      )}

      {/* Name */}
      <div className="space-y-1.5">
        <label htmlFor="agent-name" className="text-sm font-medium text-zinc-700">
          Name <span className="text-red-500">*</span>
        </label>
        <Input
          id="agent-name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g. procurement-bot"
          maxLength={255}
          required
          aria-invalid={fieldError?.includes("name") ? true : undefined}
        />
      </div>

      {/* Profile */}
      <div className="space-y-1.5">
        <label htmlFor="agent-profile" className="text-sm font-medium text-zinc-700">
          Profile {mode === "create" && <span className="text-red-500">*</span>}
        </label>
        {profiles.length > 0 ? (
          <Select value={profileId} onValueChange={(v) => setProfileId(v ?? "")}>
            <SelectTrigger id="agent-profile" className="w-full">
              <SelectValue placeholder="Select a profile" />
            </SelectTrigger>
            <SelectContent>
              {profiles.map((p) => (
                <SelectItem key={p.id} value={p.id}>
                  {p.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <Input
            id="agent-profile"
            value={profileId}
            onChange={(e) => setProfileId(e.target.value)}
            placeholder="prof_..."
          />
        )}
        <p className="text-xs text-zinc-400">
          The policy profile that governs this agent&apos;s spending rules.
        </p>
      </div>

      {/* Status (edit only) */}
      {mode === "edit" && (
        <div className="space-y-1.5">
          <label htmlFor="agent-status" className="text-sm font-medium text-zinc-700">
            Status
          </label>
          <Select value={status} onValueChange={(v) => { if (v) setStatus(v as AgentStatus); }}>
            <SelectTrigger id="agent-status" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="active">Active</SelectItem>
              <SelectItem value="suspended">Suspended</SelectItem>
              <SelectItem value="revoked">Revoked</SelectItem>
            </SelectContent>
          </Select>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2 pt-2">
        <Button type="submit" disabled={isPending}>
          {isPending && <Loader2 className="mr-1 h-3 w-3 animate-spin" data-icon="inline-start" />}
          {mode === "create" ? "Create Agent" : "Save Changes"}
        </Button>
        <Button
          type="button"
          variant="outline"
          onClick={() => router.back()}
          disabled={isPending}
        >
          Cancel
        </Button>
      </div>
    </form>
  );
}
