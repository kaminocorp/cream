import { Skeleton } from "@/components/ui/skeleton";

export default function Loading() {
  return (
    <div>
      <div className="px-6 pb-4 pt-6">
        <Skeleton className="h-7 w-32" />
        <Skeleton className="mt-2 h-4 w-80" />
      </div>
      <div className="p-6 space-y-2">
        {Array.from({ length: 10 }).map((_, i) => (
          <Skeleton key={i} className="h-11 w-full" />
        ))}
      </div>
    </div>
  );
}
