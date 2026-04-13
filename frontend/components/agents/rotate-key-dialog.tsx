"use client";

import { useState, useTransition } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ApiKeyDisplay } from "./api-key-display";
import { rotateAgentKey, RotateKeyResult } from "@/app/agents/actions";
import { KeyRound, Loader2, AlertTriangle } from "lucide-react";

interface RotateKeyDialogProps {
  agentId: string;
  agentName: string;
}

type Stage = "confirm" | "loading" | "display" | "error";

/**
 * Credential rotation dialog. Three-stage flow:
 *
 * 1. **Confirm** — warns the operator that the current key will be invalidated.
 * 2. **Display** — shows the new key exactly once via `<ApiKeyDisplay>`.
 * 3. **Error** — if the rotation fails, shows the error with a retry option.
 *
 * The dialog blocks close during the display stage — the operator must click
 * "I've copied the key" to dismiss.
 */
export function RotateKeyDialog({ agentId, agentName }: RotateKeyDialogProps) {
  const [open, setOpen] = useState(false);
  const [stage, setStage] = useState<Stage>("confirm");
  const [newKey, setNewKey] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [isPending, startTransition] = useTransition();

  const handleRotate = () => {
    setStage("loading");
    startTransition(async () => {
      const result: RotateKeyResult = await rotateAgentKey(agentId);
      if (result.ok) {
        setNewKey(result.apiKey);
        setStage("display");
      } else {
        setError(result.message);
        setStage("error");
      }
    });
  };

  const handleClose = () => {
    // Block close during key display to prevent accidental dismissal.
    if (stage === "display") return;
    setOpen(false);
  };

  const handleOpenChange = (next: boolean) => {
    if (!next && stage === "display") return;
    if (next) {
      setStage("confirm");
      setNewKey(null);
      setError("");
    }
    setOpen(next);
  };

  const handleAcknowledge = () => {
    setNewKey(null);
    setOpen(false);
    setStage("confirm");
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger
        render={
          <Button variant="outline" size="sm">
            <KeyRound className="mr-1 h-3 w-3" data-icon="inline-start" />
            Rotate Key
          </Button>
        }
      />
      <DialogContent
        showCloseButton={stage !== "display"}
        className="sm:max-w-md"
      >
        <DialogHeader>
          <DialogTitle>Rotate API Key</DialogTitle>
          <DialogDescription>
            {stage === "display"
              ? `New API key for ${agentName}`
              : `This will invalidate the current API key for ${agentName}.`}
          </DialogDescription>
        </DialogHeader>

        {stage === "confirm" && (
          <div className="space-y-4">
            <div className="flex items-start gap-2 rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-800">
              <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
              <p>
                The current key will stop working immediately. Make sure the
                agent is updated with the new key before it makes its next API
                call.
              </p>
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
              <Button variant="destructive" onClick={handleRotate}>
                Rotate Key
              </Button>
            </div>
          </div>
        )}

        {stage === "loading" && (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-zinc-400" />
          </div>
        )}

        {stage === "display" && newKey && (
          <ApiKeyDisplay apiKey={newKey} onAcknowledge={handleAcknowledge} />
        )}

        {stage === "error" && (
          <div className="space-y-4">
            <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
              {error}
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
              <Button onClick={handleRotate} disabled={isPending}>
                Retry
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
