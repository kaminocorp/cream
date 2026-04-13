"use client";

import { useState, useTransition } from "react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { WebhookForm } from "@/components/settings/webhook-form";
import { WebhookDeliveryLog } from "@/components/settings/webhook-delivery-log";
import { ProviderKeysForm } from "@/components/settings/provider-keys-form";
import { AccountSettings } from "@/components/settings/account-settings";
import { WebhookResponse, WebhookDelivery, ProviderKeyInfo } from "@/lib/types";
import { Session } from "@/lib/auth";
import {
  registerWebhook,
  deleteWebhook,
  testWebhook,
  saveProviderKey,
} from "./actions";

interface SettingsClientProps {
  webhooks: WebhookResponse[];
  providerKeys: ProviderKeyInfo[];
  session: Session | null;
}

export function SettingsClient({ webhooks, providerKeys, session }: SettingsClientProps) {
  const [localWebhooks, setLocalWebhooks] = useState(webhooks);
  const [localKeys, setLocalKeys] = useState(providerKeys);
  const [selectedWebhookId, setSelectedWebhookId] = useState<string | null>(null);
  const [deliveries, setDeliveries] = useState<WebhookDelivery[]>([]);
  const [isPending, startTransition] = useTransition();

  async function handleRegisterWebhook(url: string, secret: string, events: string[]) {
    const result = await registerWebhook(url, secret, events);
    if (result.ok) {
      // Optimistic: just re-render with server data on next navigation.
      // For now, add a placeholder.
      setLocalWebhooks((prev) => [
        ...prev,
        { id: `whk_${Date.now()}`, url, events, status: "active" },
      ]);
    }
    return result;
  }

  async function handleDeleteWebhook(id: string) {
    startTransition(async () => {
      const result = await deleteWebhook(id);
      if (result.ok) {
        setLocalWebhooks((prev) => prev.filter((w) => w.id !== id));
        if (selectedWebhookId === id) {
          setSelectedWebhookId(null);
          setDeliveries([]);
        }
      }
    });
  }

  async function handleTestWebhook(id: string) {
    startTransition(async () => {
      await testWebhook(id);
    });
  }

  async function handleViewDeliveries(webhookId: string) {
    setSelectedWebhookId(webhookId);
    try {
      const baseUrl = process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, "") ?? "";
      // Fetch deliveries client-side for interactivity.
      const res = await fetch(
        `${baseUrl}/v1/webhooks/${webhookId}/deliveries?limit=20`,
        { cache: "no-store" },
      );
      if (res.ok) {
        setDeliveries(await res.json());
      }
    } catch {
      setDeliveries([]);
    }
  }

  async function handleSaveProviderKey(providerName: string, apiKey: string) {
    const result = await saveProviderKey(providerName, apiKey);
    if (result.ok) {
      // Update local state with masked preview.
      const preview = apiKey.length >= 4 ? `...${apiKey.slice(-4)}` : "****";
      setLocalKeys((prev) => {
        const existing = prev.findIndex((k) => k.provider_name === providerName);
        const updated: ProviderKeyInfo = {
          id: existing >= 0 ? prev[existing].id : `new_${Date.now()}`,
          provider_name: providerName,
          key_preview: preview,
          created_at: existing >= 0 ? prev[existing].created_at : new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };
        if (existing >= 0) {
          const copy = [...prev];
          copy[existing] = updated;
          return copy;
        }
        return [...prev, updated];
      });
    }
    return result;
  }

  return (
    <Tabs defaultValue={0}>
      <TabsList>
        <TabsTrigger value={0}>Webhooks</TabsTrigger>
        <TabsTrigger value={1}>Provider Keys</TabsTrigger>
        <TabsTrigger value={2}>Account</TabsTrigger>
      </TabsList>

      {/* --- Webhooks Tab --- */}
      <TabsContent value={0}>
        <div className="mt-4 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Register Webhook</CardTitle>
              <CardDescription>
                Receive real-time events for payment lifecycle transitions.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <WebhookForm onRegister={handleRegisterWebhook} />
            </CardContent>
          </Card>

          {localWebhooks.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Registered Endpoints</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {localWebhooks.map((wh) => (
                  <div
                    key={wh.id}
                    className="flex items-center justify-between rounded-md border px-3 py-2"
                  >
                    <div>
                      <p className="text-sm font-mono truncate max-w-xs">{wh.url}</p>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge className="text-xs">{wh.status}</Badge>
                        <span className="text-xs text-zinc-400">
                          {wh.events.join(", ")}
                        </span>
                      </div>
                    </div>
                    <div className="flex gap-1.5">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleViewDeliveries(wh.id)}
                      >
                        Deliveries
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleTestWebhook(wh.id)}
                        disabled={isPending}
                      >
                        Test
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleDeleteWebhook(wh.id)}
                        disabled={isPending}
                      >
                        Delete
                      </Button>
                    </div>
                  </div>
                ))}
              </CardContent>
            </Card>
          )}

          {selectedWebhookId && (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Delivery Log</CardTitle>
                <CardDescription>
                  Recent deliveries for endpoint{" "}
                  <span className="font-mono text-xs">{selectedWebhookId}</span>
                </CardDescription>
              </CardHeader>
              <CardContent>
                <WebhookDeliveryLog deliveries={deliveries} />
              </CardContent>
            </Card>
          )}
        </div>
      </TabsContent>

      {/* --- Provider Keys Tab --- */}
      <TabsContent value={1}>
        <div className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Provider API Keys</CardTitle>
              <CardDescription>
                Keys are encrypted at rest (AES-256-GCM). Only the last 4
                characters are shown.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <ProviderKeysForm
                existingKeys={localKeys}
                onSave={handleSaveProviderKey}
              />
            </CardContent>
          </Card>
        </div>
      </TabsContent>

      {/* --- Account Tab --- */}
      <TabsContent value={2}>
        <div className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Account</CardTitle>
              <CardDescription>Operator identity and credentials</CardDescription>
            </CardHeader>
            <CardContent>
              <AccountSettings session={session} />
            </CardContent>
          </Card>
        </div>
      </TabsContent>
    </Tabs>
  );
}
