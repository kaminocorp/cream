import { Skeleton } from "@/components/ui/skeleton";

export default function Loading() {
  return (
    <div>
      <div className="px-6 pb-4 pt-6">
        <Skeleton className="h-7 w-36" />
        <Skeleton className="mt-2 h-4 w-48" />
      </div>
      <div className="mx-auto max-w-md space-y-5 p-6">
        <Skeleton className="h-8 w-full" />
        <Skeleton className="h-8 w-full" />
        <Skeleton className="h-8 w-full" />
        <Skeleton className="h-8 w-32" />
      </div>
    </div>
  );
}
