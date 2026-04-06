import { Header } from "@/components/layout/header";

interface PageHeaderProps {
  title: string;
  description?: string;
}

export function PageHeader({ title, description }: PageHeaderProps) {
  return <Header title={title} description={description} />;
}
