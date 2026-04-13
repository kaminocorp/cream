import { getApiClient } from "@/lib/api";
import { requireAuth } from "@/lib/auth";
import { PageHeader } from "@/components/shared/page-header";
import { SettingsClient } from "./settings-client";
import { WebhookResponse, ProviderKeyInfo } from "@/lib/types";

export default async function SettingsPage() {
  const session = await requireAuth();

  let webhooks: WebhookResponse[] = [];
  let providerKeys: ProviderKeyInfo[] = [];

  try {
    const api = await getApiClient();
    const [wh, pk] = await Promise.allSettled([
      api.listWebhooks(),
      api.getProviderKeys(),
    ]);
    if (wh.status === "fulfilled") webhooks = wh.value;
    if (pk.status === "fulfilled") providerKeys = pk.value;
  } catch {
    // Errors are non-fatal — we still render the page with empty data.
  }

  return (
    <div>
      <PageHeader title="Settings" description="Webhooks, provider keys, and account configuration" />
      <div className="p-6 max-w-2xl">
        <SettingsClient
          webhooks={webhooks}
          providerKeys={providerKeys}
          session={session}
        />
      </div>
    </div>
  );
}
