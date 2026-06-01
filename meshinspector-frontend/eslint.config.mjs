import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  {
    files: [
      "src/app/viewer/page.tsx",
      "src/features/editor/hooks/useJobEventStream.ts",
      "src/features/editor/viewer/MeshLibWorkbenchHost.tsx",
      "src/features/editor/viewer/ViewerEngine.tsx",
    ],
    rules: {
      "react-hooks/set-state-in-effect": "off",
      "react-hooks/immutability": "off",
    },
  },
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    "public/meshlib-workbench/runtime/**",
  ]),
]);

export default eslintConfig;
