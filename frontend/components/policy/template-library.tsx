"use client";

import { useState, useTransition } from "react";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { PolicyTemplate, AgentSummary } from "@/lib/types";

interface TemplateLibraryProps {
  templates: PolicyTemplate[];
  agents: AgentSummary[];
  onApply: (templateId: string, agentId: string) => Promise<{ ok: boolean; message?: string }>;
}

function categoryColor(category: string): string {
  switch (category) {
    case "starter":
      return "bg-blue-100 text-blue-800";
    case "conservative":
      return "bg-yellow-100 text-yellow-800";
    case "compliance":
      return "bg-purple-100 text-purple-800";
    default:
      return "bg-zinc-100 text-zinc-700";
  }
}

export function TemplateLibrary({ templates, agents, onApply }: TemplateLibraryProps) {
  const [applyingTo, setApplyingTo] = useState<{ templateId: string; agentId: string } | null>(
    null,
  );
  const [selectingFor, setSelectingFor] = useState<string | null>(null);
  const [result, setResult] = useState<{ ok: boolean; message?: string } | null>(null);
  const [isPending, startTransition] = useTransition();

  function handleApply(templateId: string, agentId: string) {
    setResult(null);
    setApplyingTo({ templateId, agentId });
    startTransition(async () => {
      const res = await onApply(templateId, agentId);
      setResult(res);
      setApplyingTo(null);
      setSelectingFor(null);
    });
  }

  if (templates.length === 0) {
    return <p className="text-sm text-zinc-400">No templates available.</p>;
  }

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {templates.map((t) => (
        <Card key={t.id}>
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm">{t.name}</CardTitle>
              <Badge className={categoryColor(t.category)}>{t.category}</Badge>
            </div>
            <CardDescription className="text-xs">{t.description}</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="mb-3 text-xs text-zinc-400">
              {Array.isArray(t.rules) ? t.rules.length : 0} rules
              {t.is_builtin && " · built-in"}
            </p>

            {selectingFor === t.id ? (
              <div className="space-y-2">
                <p className="text-xs font-medium">Apply to agent:</p>
                {agents.length === 0 ? (
                  <p className="text-xs text-zinc-400">No agents available.</p>
                ) : (
                  <div className="flex flex-wrap gap-1.5">
                    {agents
                      .filter((a) => a.status === "active")
                      .map((a) => (
                        <Button
                          key={a.id}
                          variant="outline"
                          size="sm"
                          className="text-xs"
                          onClick={() => handleApply(t.id, a.id)}
                          disabled={
                            isPending &&
                            applyingTo?.templateId === t.id &&
                            applyingTo?.agentId === a.id
                          }
                        >
                          {a.name}
                        </Button>
                      ))}
                  </div>
                )}
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs"
                  onClick={() => setSelectingFor(null)}
                >
                  Cancel
                </Button>
              </div>
            ) : (
              <Button
                size="sm"
                onClick={() => {
                  setSelectingFor(t.id);
                  setResult(null);
                }}
              >
                Apply
              </Button>
            )}

            {result && selectingFor === null && (
              <p
                className={`mt-2 text-xs ${result.ok ? "text-green-600" : "text-red-600"}`}
                role="alert"
              >
                {result.ok ? "Template applied successfully" : result.message}
              </p>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
