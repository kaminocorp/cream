"use client";

import { useState, useTransition, useEffect } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { register } from "../login/actions";

export default function RegisterPage() {
  const router = useRouter();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();
  const [checking, setChecking] = useState(true);

  // Check if registration is available (no operators exist yet)
  useEffect(() => {
    const baseUrl = process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, "") ?? "";
    fetch(`${baseUrl}/v1/auth/status`, { cache: "no-store" })
      .then((res) => res.json())
      .then((data: { registered: boolean }) => {
        if (data.registered) {
          // Operators already exist — registration is closed
          router.replace("/login");
        } else {
          setChecking(false);
        }
      })
      .catch(() => {
        setChecking(false);
      });
  }, [router]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      return;
    }

    startTransition(async () => {
      const result = await register(name, email, password);
      if (!result.ok) {
        setError(result.message);
      }
    });
  }

  if (checking) {
    return (
      <div className="text-center">
        <p className="text-sm text-zinc-500">Checking registration status...</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center">
        <h1 className="text-2xl font-semibold tracking-tight">cream</h1>
        <p className="mt-1 text-sm text-zinc-500">
          Create the first operator account
        </p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="space-y-2">
          <label htmlFor="name" className="text-sm font-medium">
            Name
          </label>
          <Input
            id="name"
            type="text"
            placeholder="Your name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            autoComplete="name"
            autoFocus
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="email" className="text-sm font-medium">
            Email
          </label>
          <Input
            id="email"
            type="email"
            placeholder="admin@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
            autoComplete="email"
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="password" className="text-sm font-medium">
            Password
          </label>
          <Input
            id="password"
            type="password"
            placeholder="Min 12 characters"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={12}
            autoComplete="new-password"
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="confirm-password" className="text-sm font-medium">
            Confirm password
          </label>
          <Input
            id="confirm-password"
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
            minLength={12}
            autoComplete="new-password"
          />
        </div>

        {error && (
          <p className="text-sm text-red-600" role="alert">
            {error}
          </p>
        )}

        <Button type="submit" className="w-full" disabled={isPending}>
          {isPending ? "Creating account..." : "Create account"}
        </Button>
      </form>

      <p className="text-center text-xs text-zinc-400">
        Already registered?{" "}
        <Link href="/login" className="text-zinc-700 underline hover:text-zinc-900">
          Sign in
        </Link>
      </p>
    </div>
  );
}
