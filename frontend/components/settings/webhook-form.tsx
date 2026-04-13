"use client";

import { useState, useTransition } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface WebhookFormProps {
  onRegister: (url: string, secret: string, events: string[]) => Promise<{ ok: boolean; message?: string }>;
}

export function WebhookForm({ onRegister }: WebhookFormProps) {
  const [url, setUrl] = useState("");
  const [secret, setSecret] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    startTransition(async () => {
      const result = await onRegister(url, secret, ["*"]);
      if (!result.ok) {
        setError(result.message ?? "Failed to register webhook");
      } else {
        setUrl("");
        setSecret("");
      }
    });
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <div className="space-y-1.5">
        <label htmlFor="webhook-url" className="text-sm font-medium">
          Endpoint URL
        </label>
        <Input
          id="webhook-url"
          type="url"
          placeholder="https://your-service.com/webhooks/cream"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          required
        />
      </div>
      <div className="space-y-1.5">
        <label htmlFor="webhook-secret" className="text-sm font-medium">
          Signing Secret
        </label>
        <Input
          id="webhook-secret"
          type="password"
          placeholder="Min 16 characters"
          value={secret}
          onChange={(e) => setSecret(e.target.value)}
          required
          minLength={16}
        />
      </div>
      {error && (
        <p className="text-sm text-red-600" role="alert">
          {error}
        </p>
      )}
      <Button type="submit" disabled={isPending}>
        {isPending ? "Registering..." : "Register Webhook"}
      </Button>
    </form>
  );
}
