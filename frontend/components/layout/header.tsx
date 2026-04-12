import type { ReactNode } from "react";
import { Separator } from "@/components/ui/separator";

interface HeaderProps {
  title: string;
  /**
   * Subtitle content. Accepts a plain string or arbitrary React nodes
   * (e.g., inline badges, status pills, monospace IDs). Widened from
   * `string` in Phase 15.2 to let the agent detail page render an
   * inline status + profile line.
   */
  description?: ReactNode;
}

export function Header({ title, description }: HeaderProps) {
  return (
    <div className="px-6 py-4">
      <h1 className="text-xl font-semibold">{title}</h1>
      {description && (
        <div className="mt-0.5 text-sm text-zinc-500">{description}</div>
      )}
      <Separator className="mt-4" />
    </div>
  );
}
