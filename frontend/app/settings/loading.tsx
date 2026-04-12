import { Skeleton } from "@/components/ui/skeleton";

export default function Loading() {
  return (
    <div>
      <div className="px-6 pb-4 pt-6">
        <Skeleton className="h-7 w-24" />
        <Skeleton className="mt-2 h-4 w-64" />
      </div>
      <div className="max-w-xl space-y-4 p-6">
        <Skeleton className="h-40 w-full rounded-xl" />
      </div>
    </div>
  );
}
