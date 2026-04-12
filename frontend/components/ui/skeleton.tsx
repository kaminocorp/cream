import { cn } from "@/lib/utils";

/**
 * shadcn/ui-compatible skeleton primitive. Minimal implementation: a div
 * with a pulse animation and rounded corners. Use for loading fallbacks
 * in `loading.tsx` files and Suspense boundaries.
 *
 * Matches the shadcn v4 canonical implementation so it can be swapped for
 * the `shadcn add skeleton` output later without any consumer churn.
 */
function Skeleton({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("animate-pulse rounded-md bg-zinc-100", className)}
      {...props}
    />
  );
}

export { Skeleton };
