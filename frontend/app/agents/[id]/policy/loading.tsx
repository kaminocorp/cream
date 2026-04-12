import { Skeleton } from "@/components/ui/skeleton";

export default function Loading() {
  return (
    <div>
      <div className="px-6 pb-4 pt-6">
        <Skeleton className="h-7 w-40" />
        <Skeleton className="mt-2 h-4 w-80" />
      </div>
      <div className="space-y-4 p-6">
        <Skeleton className="h-8 w-64" />
        <div className="space-y-3">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      </div>
    </div>
  );
}
