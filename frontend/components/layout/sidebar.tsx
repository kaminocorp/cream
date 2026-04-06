"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import {
  LayoutDashboard, ArrowLeftRight, Users, ShieldCheck,
  AlertTriangle, FileText, Activity, Settings,
} from "lucide-react";

const nav = [
  { href: "/",             label: "Overview",     icon: LayoutDashboard },
  { href: "/transactions", label: "Transactions", icon: ArrowLeftRight  },
  { href: "/escalations",  label: "Escalations",  icon: AlertTriangle   },
  { href: "/agents",       label: "Agents",       icon: Users           },
  { href: "/policies",     label: "Policies",     icon: ShieldCheck     },
  { href: "/audit",        label: "Audit Log",    icon: FileText        },
  { href: "/providers",    label: "Providers",    icon: Activity        },
  { href: "/settings",     label: "Settings",     icon: Settings        },
];

export function Sidebar() {
  const pathname = usePathname();
  return (
    <aside className="flex h-screen w-56 flex-col border-r bg-zinc-50 px-3 py-4 shrink-0">
      <div className="mb-6 px-2">
        <span className="text-lg font-semibold tracking-tight">cream</span>
        <span className="ml-2 text-xs text-zinc-400">control plane</span>
      </div>
      <nav className="flex flex-col gap-1">
        {nav.map(({ href, label, icon: Icon }) => (
          <Link
            key={href}
            href={href}
            className={cn(
              "flex items-center gap-2.5 rounded-md px-2.5 py-2 text-sm transition-colors",
              pathname === href
                ? "bg-zinc-900 text-white"
                : "text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900",
            )}
          >
            <Icon className="h-4 w-4 shrink-0" />
            {label}
          </Link>
        ))}
      </nav>
    </aside>
  );
}
