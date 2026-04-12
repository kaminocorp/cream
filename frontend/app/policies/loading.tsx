import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export default function Loading() {
  return (
    <div>
      <div className="px-6 pb-4 pt-6">
        <Skeleton className="h-7 w-28" />
        <Skeleton className="mt-2 h-4 w-72" />
      </div>
      <div className="p-6">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <Card key={i}>
              <CardHeader className="pb-2">
                <Skeleton className="h-4 w-32" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-3 w-40" />
                <Skeleton className="mt-1 h-3 w-56" />
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
