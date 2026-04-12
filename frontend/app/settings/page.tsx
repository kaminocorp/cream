import { PageHeader } from "@/components/shared/page-header";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

export default function SettingsPage() {
  return (
    <div>
      <PageHeader title="Settings" description="Webhook endpoints and operator configuration" />
      <div className="p-6 max-w-xl space-y-6">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Register Webhook Endpoint</CardTitle>
            <CardDescription>
              Receive real-time events for payment lifecycle transitions.
              HTTPS required in production.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div>
              <label className="text-sm font-medium">Endpoint URL</label>
              <Input placeholder="https://your-service.com/webhooks/cream" disabled />
            </div>
            <div>
              <label className="text-sm font-medium">Signing Secret</label>
              <Input type="password" placeholder="Minimum 16 characters" disabled />
            </div>
            <Button disabled>Register (Phase 16-A)</Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
