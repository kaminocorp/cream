"use client";

import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  LayoutDashboard, ArrowLeftRight, Users, ShieldCheck,
  AlertTriangle, FileText, Activity, Settings, Menu, X,
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

function isActive(pathname: string, href: string): boolean {
  if (href === "/") return pathname === "/";
  return pathname === href || pathname.startsWith(href + "/");
}

export function Sidebar() {
  const pathname = usePathname();
  const [mobileOpen, setMobileOpen] = useState(false);

  const navItems = (
    <>
      <div className="mb-6 px-2">
        <span className="text-lg font-semibold tracking-tight">cream</span>
        <span className="ml-2 text-xs text-zinc-400">control plane</span>
      </div>
      <nav className="flex flex-col gap-1">
        {nav.map(({ href, label, icon: Icon }) => (
          <Link
            key={href}
            href={href}
            onClick={() => setMobileOpen(false)}
            className={cn(
              "flex items-center gap-2.5 rounded-md px-2.5 py-2 text-sm transition-colors",
              isActive(pathname, href)
                ? "bg-zinc-900 text-white"
                : "text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900",
            )}
          >
            <Icon className="h-4 w-4 shrink-0" />
            {label}
          </Link>
        ))}
      </nav>
    </>
  );

  return (
    <>
      {/* Desktop sidebar */}
      <aside className="hidden h-screen w-56 shrink-0 flex-col border-r bg-zinc-50 px-3 py-4 lg:flex">
        {navItems}
      </aside>

      {/* Mobile toggle */}
      <div className="fixed left-3 top-3 z-50 lg:hidden">
        <Button
          variant="outline"
          size="icon"
          onClick={() => setMobileOpen(!mobileOpen)}
          aria-label={mobileOpen ? "Close menu" : "Open menu"}
        >
          {mobileOpen ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
        </Button>
      </div>

      {/* Mobile overlay */}
      {mobileOpen && (
        <>
          <div
            className="fixed inset-0 z-40 bg-black/20 lg:hidden"
            onClick={() => setMobileOpen(false)}
          />
          <aside className="fixed inset-y-0 left-0 z-40 flex w-56 flex-col border-r bg-zinc-50 px-3 py-4 pt-14 lg:hidden">
            {navItems}
          </aside>
        </>
      )}
    </>
  );
}
