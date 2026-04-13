"use server";

import { redirect } from "next/navigation";
import { setAuthCookies, clearAuthCookies } from "@/lib/auth";

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

export type AuthResult =
  | { ok: true }
  | { ok: false; message: string };

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

function validateEmail(email: string): string | null {
  if (!email.trim()) return "Email is required";
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim())) return "Invalid email format";
  return null;
}

function validatePassword(password: string): string | null {
  if (!password) return "Password is required";
  if (password.length < 12) return "Password must be at least 12 characters";
  return null;
}

function validateName(name: string): string | null {
  if (!name.trim()) return "Name is required";
  if (name.trim().length > 200) return "Name must be 200 characters or fewer";
  return null;
}

// ---------------------------------------------------------------------------
// API helpers
// ---------------------------------------------------------------------------

function getBaseUrl(): string {
  const url = process.env.NEXT_PUBLIC_API_URL;
  if (!url) throw new Error("NEXT_PUBLIC_API_URL is required");
  return url.replace(/\/$/, "");
}

async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${getBaseUrl()}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify(body),
    cache: "no-store",
    signal: AbortSignal.timeout(15_000),
  });

  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try {
      const err = await res.json();
      msg = err.message ?? err.error ?? msg;
    } catch { /* use status */ }
    throw new Error(msg);
  }

  return res.json() as Promise<T>;
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

interface AuthTokens {
  access_token: string;
  refresh_token: string;
}

/**
 * Login with email + password. Sets httpOnly cookies on success.
 */
export async function login(email: string, password: string): Promise<AuthResult> {
  const emailErr = validateEmail(email);
  if (emailErr) return { ok: false, message: emailErr };
  if (!password) return { ok: false, message: "Password is required" };

  try {
    const data = await apiPost<AuthTokens>("/v1/auth/login", {
      email: email.trim(),
      password,
    });

    await setAuthCookies(data.access_token, data.refresh_token);
  } catch (err) {
    const message = err instanceof Error ? err.message : "Login failed";
    return { ok: false, message };
  }

  redirect("/");
}

/**
 * Register the first operator. Only works when no operators exist.
 */
export async function register(
  name: string,
  email: string,
  password: string,
): Promise<AuthResult> {
  const nameErr = validateName(name);
  if (nameErr) return { ok: false, message: nameErr };
  const emailErr = validateEmail(email);
  if (emailErr) return { ok: false, message: emailErr };
  const passErr = validatePassword(password);
  if (passErr) return { ok: false, message: passErr };

  try {
    const data = await apiPost<AuthTokens & { operator_id: string }>("/v1/auth/register", {
      name: name.trim(),
      email: email.trim(),
      password,
    });

    await setAuthCookies(data.access_token, data.refresh_token);
  } catch (err) {
    const message = err instanceof Error ? err.message : "Registration failed";
    return { ok: false, message };
  }

  redirect("/");
}

/**
 * Logout: revoke refresh token on backend, clear cookies.
 */
export async function logout(): Promise<void> {
  try {
    const { getRefreshToken } = await import("@/lib/auth");
    const refreshToken = await getRefreshToken();
    if (refreshToken) {
      await apiPost("/v1/auth/logout", { refresh_token: refreshToken }).catch(() => {});
    }
  } catch { /* best-effort */ }

  await clearAuthCookies();
  redirect("/login");
}
