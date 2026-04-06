import { Separator } from "@/components/ui/separator";

interface HeaderProps {
  title: string;
  description?: string;
}

export function Header({ title, description }: HeaderProps) {
  return (
    <div className="px-6 py-4">
      <h1 className="text-xl font-semibold">{title}</h1>
      {description && (
        <p className="mt-0.5 text-sm text-zinc-500">{description}</p>
      )}
      <Separator className="mt-4" />
    </div>
  );
}
