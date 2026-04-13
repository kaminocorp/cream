import { cookies } from "next/headers";
import { redirect } from "next/navigation";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Session {
  operatorId: string;
  email: string;
  role: string;
  exp: number;
}

// ---------------------------------------------------------------------------
// Cookie constants
// ---------------------------------------------------------------------------

const ACCESS_COOKIE = "cream_access";
const REFRESH_COOKIE = "cream_refresh";

/** Access token TTL in seconds (matches backend JWT_ACCESS_TTL_SECS default). */
const ACCESS_TTL = 900; // 15 minutes
/** Refresh token TTL in seconds (matches backend JWT_REFRESH_TTL_SECS default). */
const REFRESH_TTL = 604800; // 7 days

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Decode a JWT payload without signature verification.
 *
 * We intentionally skip verification here because the Rust backend is the
 * source of truth for token validity. The frontend only needs to read claims
 * for display (operator email, role) and to detect expiry for proactive
 * refresh. Every API call still sends the raw token to the backend which
 * performs full cryptographic verification.
 */
function decodeJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;
    const payload = Buffer.from(parts[1], "base64url").toString("utf-8");
    return JSON.parse(payload);
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Read the current session from the access token cookie.
 * Returns null if no valid session exists (no cookie or malformed token).
 * Does NOT verify signature — that's the backend's job.
 */
export async function getSession(): Promise<Session | null> {
  const cookieStore = await cookies();
  const accessCookie = cookieStore.get(ACCESS_COOKIE);
  if (!accessCookie?.value) return null;

  const claims = decodeJwtPayload(accessCookie.value);
  if (!claims) return null;

  const sub = claims.sub as string | undefined;
  const email = claims.email as string | undefined;
  const role = claims.role as string | undefined;
  const exp = claims.exp as number | undefined;

  if (!sub || !email || !role || !exp) return null;

  return { operatorId: sub, email, role, exp };
}

/**
 * Require an authenticated session. Redirects to /login if none exists.
 * Call this at the top of protected server components/pages.
 */
export async function requireAuth(): Promise<Session> {
  const session = await getSession();
  if (!session) {
    redirect("/login");
  }
  return session;
}

/**
 * Check if the access token is expired or about to expire (within 60s).
 */
export function isTokenExpired(session: Session): boolean {
  const nowSecs = Math.floor(Date.now() / 1000);
  return session.exp <= nowSecs + 60; // 60s buffer
}

/**
 * Attempt to refresh the session using the refresh token cookie.
 * On success, sets new cookies and returns the new session.
 * On failure, returns null (caller should redirect to /login).
 */
export async function refreshSession(): Promise<Session | null> {
  const cookieStore = await cookies();
  const refreshCookie = cookieStore.get(REFRESH_COOKIE);
  if (!refreshCookie?.value) return null;

  const baseUrl = process.env.NEXT_PUBLIC_API_URL;
  if (!baseUrl) return null;

  try {
    const res = await fetch(`${baseUrl.replace(/\/$/, "")}/v1/auth/refresh`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
      },
      body: JSON.stringify({ refresh_token: refreshCookie.value }),
      cache: "no-store",
      signal: AbortSignal.timeout(10_000),
    });

    if (!res.ok) return null;

    const data = (await res.json()) as {
      access_token: string;
      refresh_token: string;
    };

    // Set updated cookies
    const secure = process.env.NODE_ENV === "production";

    cookieStore.set(ACCESS_COOKIE, data.access_token, {
      httpOnly: true,
      secure,
      sameSite: "lax",
      path: "/",
      maxAge: ACCESS_TTL,
    });

    cookieStore.set(REFRESH_COOKIE, data.refresh_token, {
      httpOnly: true,
      secure,
      sameSite: "lax",
      path: "/",
      maxAge: REFRESH_TTL,
    });

    const claims = decodeJwtPayload(data.access_token);
    if (!claims) return null;

    // Validate individual claims — same checks as getSession(). Without
    // this, a malformed token would produce a Session with undefined fields
    // that cause runtime errors downstream.
    const sub = claims.sub as string | undefined;
    const email = claims.email as string | undefined;
    const role = claims.role as string | undefined;
    const exp = claims.exp as number | undefined;

    if (!sub || !email || !role || !exp) return null;

    return { operatorId: sub, email, role, exp };
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Cookie setters (for use in server actions)
// ---------------------------------------------------------------------------

/**
 * Store tokens in httpOnly cookies after successful login/register.
 */
export async function setAuthCookies(
  accessToken: string,
  refreshToken: string,
): Promise<void> {
  const cookieStore = await cookies();
  const secure = process.env.NODE_ENV === "production";

  cookieStore.set(ACCESS_COOKIE, accessToken, {
    httpOnly: true,
    secure,
    sameSite: "lax",
    path: "/",
    maxAge: ACCESS_TTL,
  });

  cookieStore.set(REFRESH_COOKIE, refreshToken, {
    httpOnly: true,
    secure,
    sameSite: "lax",
    path: "/",
    maxAge: REFRESH_TTL,
  });
}

/**
 * Clear auth cookies (logout).
 */
export async function clearAuthCookies(): Promise<void> {
  const cookieStore = await cookies();
  cookieStore.delete(ACCESS_COOKIE);
  cookieStore.delete(REFRESH_COOKIE);
}

/**
 * Get the raw access token from the cookie (for API calls).
 * Returns null if no cookie is set.
 */
export async function getAccessToken(): Promise<string | null> {
  const cookieStore = await cookies();
  return cookieStore.get(ACCESS_COOKIE)?.value ?? null;
}

/**
 * Get the raw refresh token from the cookie.
 */
export async function getRefreshToken(): Promise<string | null> {
  const cookieStore = await cookies();
  return cookieStore.get(REFRESH_COOKIE)?.value ?? null;
}
