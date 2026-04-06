import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Strict mode enabled for catching double-render bugs early.
  reactStrictMode: true,
  // Type-check during production builds (catches type regressions before deploy).
  typescript: {
    ignoreBuildErrors: false,
  },
};

export default nextConfig;
