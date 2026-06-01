import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // The current viewer stack relies on imperative WebGL and iframe bridge flows
  // that are not compatible with the React Compiler rule set yet.
  reactCompiler: false,
};

export default nextConfig;
