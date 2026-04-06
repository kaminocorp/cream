import { PageHeader } from "@/components/shared/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/shared/empty-state";
import { BarChart2 } from "lucide-react";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function AgentDetailPage({ params }: Props) {
  const { id } = await params;
  // Phase 15: const { agent, profile } = await getApiClient().getAgentPolicy(id)

  return (
    <div>
      <PageHeader
        title="Agent Detail"
        description={`Agent ${id}`}
      />
      <div className="p-6 space-y-6">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {["Daily", "Weekly", "Monthly", "Per Transaction"].map((label) => (
            <Card key={label}>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-zinc-500">{label} Limit</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-lg font-semibold">—</div>
                <div className="mt-1 h-1.5 rounded-full bg-zinc-100">
                  <div className="h-full w-0 rounded-full bg-zinc-900" />
                </div>
                <p className="mt-1 text-xs text-zinc-400">0% used</p>
              </CardContent>
            </Card>
          ))}
        </div>
        <EmptyState
          icon={BarChart2}
          title="No recent transactions"
          description="Recent payments for this agent will appear here."
        />
      </div>
    </div>
  );
}
