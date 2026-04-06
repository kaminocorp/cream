import { PageHeader } from "@/components/shared/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ArrowLeftRight, Users, DollarSign, AlertTriangle } from "lucide-react";

const summaryCards = [
  { title: "Total Payments",  value: "—", icon: ArrowLeftRight, description: "All time"           },
  { title: "Active Agents",   value: "—", icon: Users,          description: "Current"            },
  { title: "Total Spend",     value: "—", icon: DollarSign,     description: "All currencies"     },
  { title: "Pending Review",  value: "—", icon: AlertTriangle,  description: "Awaiting approval"  },
];

export default function DashboardPage() {
  return (
    <div>
      <PageHeader title="Overview" description="Payment control plane summary" />
      <div className="p-6">
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          {summaryCards.map((card) => (
            <Card key={card.title}>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <CardTitle className="text-sm font-medium text-zinc-500">{card.title}</CardTitle>
                <card.icon className="h-4 w-4 text-zinc-400" />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{card.value}</div>
                <p className="text-xs text-zinc-400 mt-0.5">{card.description}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
