import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Emit a self-contained server bundle (.next/standalone) so the Docker runtime
  // stage can ship a minimal Node server without the full node_modules tree.
  output: "standalone",
  // The current viewer stack relies on imperative WebGL and iframe bridge flows
  // that are not compatible with the React Compiler rule set yet.
  reactCompiler: false,
};

export default nextConfig;
