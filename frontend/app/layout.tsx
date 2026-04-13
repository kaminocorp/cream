import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { Sidebar } from "@/components/layout/sidebar";
import { getSession } from "@/lib/auth";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Cream — Payment Control Plane",
  description: "Operator dashboard for the Cream AI agent payment control plane",
};

/**
 * Force every route to render dynamically at request time.
 *
 * Required because `getSession()` reads cookies (request-scoped) and
 * `getApiClient()` reads env vars not available at build time.
 */
export const dynamic = "force-dynamic";

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  const session = await getSession();

  return (
    <html lang="en">
      <body className={`${inter.className} antialiased`}>
        {session ? (
          <div className="flex min-h-screen">
            <Sidebar operatorEmail={session.email} operatorRole={session.role} />
            <main className="flex-1 overflow-auto">
              {children}
            </main>
          </div>
        ) : (
          children
        )}
      </body>
    </html>
  );
}
