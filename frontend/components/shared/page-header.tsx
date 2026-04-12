import type { ReactNode } from "react";
import { Header } from "@/components/layout/header";

interface PageHeaderProps {
  title: string;
  description?: ReactNode;
}

export function PageHeader({ title, description }: PageHeaderProps) {
  return <Header title={title} description={description} />;
}
