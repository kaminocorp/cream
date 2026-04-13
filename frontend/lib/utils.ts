import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import { PaymentStatus } from "./types";

// shadcn/ui standard helper.
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// Format a Decimal-as-string with currency symbol.
// Always display as string — never parse to float.
export function formatAmount(amount: string, currency: string): string {
  if (!amount || !currency) return "—";
  const fiatSymbols: Record<string, string> = {
    USD: "$", EUR: "€", GBP: "£", SGD: "S$", JPY: "¥", AUD: "A$", CAD: "C$",
  };
  const symbol = fiatSymbols[currency] ?? "";
  return `${symbol}${amount} ${currency}`;
}

// ISO 8601 → "Apr 6, 2026, 14:32"
export function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("en-SG", {
    year: "numeric", month: "short", day: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}

// ISO 8601 → "3 minutes ago"
export function relativeTime(iso: string): string {
  const diff = Math.max(0, Date.now() - new Date(iso).getTime());
  const mins = Math.floor(diff / 60_000);
  if (mins < 1)   return "just now";
  if (mins < 60)  return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24)   return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

// Map PaymentStatus to a tailwind color string for use in status-badge.
export function statusColor(status: PaymentStatus): string {
  switch (status) {
    case "settled":          return "bg-green-100 text-green-800";
    case "submitted":        return "bg-blue-100 text-blue-800";
    case "pending_approval": return "bg-yellow-100 text-yellow-800";
    case "pending":
    case "validating":
    case "approved":         return "bg-zinc-100 text-zinc-700";
    case "failed":
    case "timed_out":        return "bg-red-100 text-red-800";
    case "blocked":          return "bg-orange-100 text-orange-800";
    case "rejected":         return "bg-red-100 text-red-700";
    default:                 return "bg-zinc-100 text-zinc-600";
  }
}
