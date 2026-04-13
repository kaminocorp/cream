"use client";

import { Session } from "@/lib/auth";
import { Badge } from "@/components/ui/badge";

interface AccountSettingsProps {
  session: Session | null;
}

export function AccountSettings({ session }: AccountSettingsProps) {
  if (!session) {
    return (
      <div className="rounded-md border p-4">
        <p className="text-sm text-zinc-500">
          Authenticated via legacy API key. Register an operator account to
          unlock per-user identity, audit attribution, and password management.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="rounded-md border p-4 space-y-3">
        <div>
          <p className="text-xs text-zinc-400">Email</p>
          <p className="text-sm font-medium">{session.email}</p>
        </div>
        <div>
          <p className="text-xs text-zinc-400">Role</p>
          <Badge className="capitalize">{session.role}</Badge>
        </div>
        <div>
          <p className="text-xs text-zinc-400">Operator ID</p>
          <p className="text-sm font-mono text-zinc-500">{session.operatorId}</p>
        </div>
      </div>
      <p className="text-xs text-zinc-400">
        Password change will be available in a future update.
      </p>
    </div>
  );
}
