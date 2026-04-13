"use client";

import { useState, useTransition } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ProviderKeyInfo } from "@/lib/types";
import { formatDate } from "@/lib/utils";

const PROVIDERS = [
  { name: "stripe", label: "Stripe", placeholder: "sk_live_..." },
  { name: "airwallex", label: "Airwallex", placeholder: "..." },
  { name: "coinbase", label: "Coinbase", placeholder: "..." },
] as const;

interface ProviderKeysFormProps {
  existingKeys: ProviderKeyInfo[];
  onSave: (providerName: string, apiKey: string) => Promise<{ ok: boolean; message?: string }>;
}

export function ProviderKeysForm({ existingKeys, onSave }: ProviderKeysFormProps) {
  const [editingProvider, setEditingProvider] = useState<string | null>(null);
  const [keyValue, setKeyValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  function getExistingKey(providerName: string): ProviderKeyInfo | undefined {
    return existingKeys.find((k) => k.provider_name === providerName);
  }

  function handleSave(providerName: string) {
    setError(null);
    startTransition(async () => {
      const result = await onSave(providerName, keyValue);
      if (!result.ok) {
        setError(result.message ?? "Failed to save key");
      } else {
        setEditingProvider(null);
        setKeyValue("");
      }
    });
  }

  return (
    <div className="space-y-4">
      {PROVIDERS.map((provider) => {
        const existing = getExistingKey(provider.name);
        const isEditing = editingProvider === provider.name;

        return (
          <div key={provider.name} className="rounded-md border p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">{provider.label}</p>
                {existing && !isEditing && (
                  <p className="mt-0.5 text-xs text-zinc-400">
                    Key: <span className="font-mono">{existing.key_preview}</span>
                    {" · "}Updated {formatDate(existing.updated_at)}
                  </p>
                )}
                {!existing && !isEditing && (
                  <p className="mt-0.5 text-xs text-zinc-400">No key configured</p>
                )}
              </div>
              {!isEditing && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setEditingProvider(provider.name);
                    setKeyValue("");
                    setError(null);
                  }}
                >
                  {existing ? "Update" : "Add Key"}
                </Button>
              )}
            </div>

            {isEditing && (
              <div className="mt-3 space-y-2">
                <Input
                  type="password"
                  placeholder={provider.placeholder}
                  value={keyValue}
                  onChange={(e) => setKeyValue(e.target.value)}
                  autoFocus
                />
                {error && (
                  <p className="text-sm text-red-600" role="alert">
                    {error}
                  </p>
                )}
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    onClick={() => handleSave(provider.name)}
                    disabled={isPending || !keyValue.trim()}
                  >
                    {isPending ? "Saving..." : "Save"}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      setEditingProvider(null);
                      setKeyValue("");
                      setError(null);
                    }}
                    disabled={isPending}
                  >
                    Cancel
                  </Button>
                </div>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
