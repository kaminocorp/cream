import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

/**
 * Suspense fallback for the Overview page. Four placeholder cards matching
 * the real layout so the shell doesn't reflow when real content lands.
 * Rendered by Next 16's implicit Suspense boundary while the async server
 * component awaits its `Promise.all`.
 */
export default function Loading() {
  return (
    <div>
      <div className="px-6 pb-4 pt-6">
        <Skeleton className="h-7 w-40" />
        <Skeleton className="mt-2 h-4 w-64" />
      </div>
      <div className="p-6">
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Card key={i}>
              <CardHeader className="flex flex-row items-center justify-between pb-2">
                <Skeleton className="h-4 w-24" />
                <Skeleton className="h-4 w-4 rounded-full" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-8 w-16" />
                <Skeleton className="mt-1 h-3 w-28" />
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
