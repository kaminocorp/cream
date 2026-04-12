import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { Sidebar } from "@/components/layout/sidebar";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Cream — Payment Control Plane",
  description: "Operator dashboard for the Cream AI agent payment control plane",
};

/**
 * Force every dashboard route to render dynamically at request time.
 *
 * Without this, Next tries to prerender the pages at build time — and
 * since `getApiClient()` reads env vars that aren't populated during
 * build, every page's `Promise.all` fails with "NEXT_PUBLIC_API_URL is
 * required". Marking the root layout as dynamic cascades to all child
 * routes so nothing is prerendered.
 *
 * Applies to the "previous caching model" only (no `cacheComponents`
 * in next.config.ts). When we eventually flip to Cache Components,
 * this export will be removed and each page will opt into freshness via
 * `"use cache"` + `cacheLife` instead.
 */
export const dynamic = "force-dynamic";

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className={`${inter.className} antialiased`}>
        <div className="flex min-h-screen">
          <Sidebar />
          <main className="flex-1 overflow-auto">
            {children}
          </main>
        </div>
      </body>
    </html>
  );
}
