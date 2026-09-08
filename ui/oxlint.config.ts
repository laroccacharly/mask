import { defineConfig } from "oxlint"

export default defineConfig({
  plugins: ["eslint", "typescript", "unicorn", "oxc", "react"],
  categories: {
    correctness: "error",
  },
  env: {
    browser: true,
  },
  ignorePatterns: ["dist"],
  rules: {
    "react/only-export-components": [
      "error",
      {
        allowConstantExport: true,
      },
    ],
  },
  overrides: [
    {
      files: ["*.config.ts", "*.config.js"],
      env: {
        node: true,
      },
    },
    {
      files: ["src/components/ui/**"],
      rules: {
        "react/only-export-components": "off",
      },
    },
  ],
})
