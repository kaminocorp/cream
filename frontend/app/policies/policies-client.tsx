"use client";

import Link from "next/link";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { EmptyState } from "@/components/shared/empty-state";
import { TemplateLibrary } from "@/components/policy/template-library";
import { AgentSummary, PolicyTemplate } from "@/lib/types";
import { applyTemplate } from "./actions";
import { ShieldCheck } from "lucide-react";

interface PoliciesClientProps {
  agents: AgentSummary[];
  templates: PolicyTemplate[];
}

export function PoliciesClient({ agents, templates }: PoliciesClientProps) {
  async function handleApplyTemplate(templateId: string, agentId: string) {
    return applyTemplate(templateId, agentId);
  }

  return (
    <Tabs defaultValue={0}>
      <TabsList>
        <TabsTrigger value={0}>Agent Policies</TabsTrigger>
        <TabsTrigger value={1}>Templates</TabsTrigger>
      </TabsList>

      <TabsContent value={0}>
        <div className="mt-4">
          {agents.length === 0 ? (
            <EmptyState
              icon={ShieldCheck}
              title="No policies to show"
              description="Create an agent first — policies are attached to agent profiles."
            />
          ) : (
            <>
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {agents.map((a) => (
                  <Link key={a.id} href={`/agents/${a.id}/policy`}>
                    <Card className="transition-colors hover:border-zinc-300">
                      <CardHeader className="pb-2">
                        <CardTitle className="flex items-center justify-between text-sm">
                          <span>{a.name}</span>
                          <Badge>{a.status}</Badge>
                        </CardTitle>
                      </CardHeader>
                      <CardContent className="text-xs text-zinc-500">
                        <div>profile: {a.profile_name}</div>
                        <div className="mt-1 font-mono">{a.id}</div>
                      </CardContent>
                    </Card>
                  </Link>
                ))}
              </div>
              <p className="mt-6 text-xs text-zinc-400">
                Select an agent to edit its profile settings and view policy rules.
              </p>
            </>
          )}
        </div>
      </TabsContent>

      <TabsContent value={1}>
        <div className="mt-4">
          <TemplateLibrary
            templates={templates}
            agents={agents}
            onApply={handleApplyTemplate}
          />
        </div>
      </TabsContent>
    </Tabs>
  );
}
