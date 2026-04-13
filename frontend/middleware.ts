import { NextRequest, NextResponse } from "next/server";

/**
 * Middleware: CSRF defense-in-depth for state-changing requests.
 *
 * Next.js 16 server actions already verify the Origin header against the
 * Host header automatically. This middleware adds an explicit check for
 * all non-GET/HEAD/OPTIONS requests as defense-in-depth, covering any
 * custom API route handlers or edge cases.
 *
 * Requests without an Origin header (same-origin browser navigations,
 * non-browser clients) are allowed through — the backend performs its
 * own auth verification.
 */
export function middleware(request: NextRequest) {
  const method = request.method.toUpperCase();

  // Only check state-changing methods.
  if (method === "GET" || method === "HEAD" || method === "OPTIONS") {
    return NextResponse.next();
  }

  const origin = request.headers.get("origin");
  if (!origin) {
    // No Origin header — same-origin navigation or non-browser client.
    return NextResponse.next();
  }

  // Compare origin against the request's host.
  const url = request.nextUrl;
  const expectedOrigin = `${url.protocol}//${url.host}`;

  if (origin !== expectedOrigin) {
    return new NextResponse("Forbidden: cross-origin request", {
      status: 403,
    });
  }

  return NextResponse.next();
}

export const config = {
  // Apply to all routes except static assets and Next.js internals.
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
