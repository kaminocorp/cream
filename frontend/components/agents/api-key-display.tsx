"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Check, Copy, AlertTriangle } from "lucide-react";

interface ApiKeyDisplayProps {
  apiKey: string;
  onAcknowledge: () => void;
}

/**
 * One-time API key display. Shows the plaintext key in a monospace box with
 * a copy-to-clipboard button and a warning that it cannot be retrieved again.
 *
 * The "I've copied it" button is the only way to dismiss — preventing
 * accidental loss of the key.
 */
export function ApiKeyDisplay({ apiKey, onAcknowledge }: ApiKeyDisplayProps) {
  const [copied, setCopied] = useState(false);
  const [copyFailed, setCopyFailed] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(apiKey);
      setCopied(true);
      setCopyFailed(false);
      setTimeout(() => setCopied(false), 2_000);
    } catch {
      setCopyFailed(true);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
        <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
        <p>
          This is the <strong>only time</strong> you will see this API key. Copy
          it now and store it securely. It cannot be retrieved again.
        </p>
      </div>

      <div className="flex items-center gap-2">
        <code className="flex-1 rounded-md border bg-zinc-50 px-3 py-2 font-mono text-xs break-all select-all">
          {apiKey}
        </code>
        <Button
          variant="outline"
          size="icon"
          onClick={handleCopy}
          aria-label="Copy API key"
        >
          {copied ? (
            <Check className="h-4 w-4 text-green-600" />
          ) : (
            <Copy className="h-4 w-4" />
          )}
        </Button>
      </div>

      {copyFailed && (
        <p className="text-xs text-red-600">
          Clipboard access failed. Please select the key above and copy manually.
        </p>
      )}

      <Button className="w-full" onClick={onAcknowledge}>
        I&apos;ve copied the key
      </Button>
    </div>
  );
}
