/* Minimal ESLint config for visualizer-v4 TypeScript/React code */
module.exports = {
  root: true,
  env: {
    browser: true,
    es2021: true,
    node: true,
  },
  parser: "@typescript-eslint/parser",
  parserOptions: {
    ecmaVersion: 2022,
    sourceType: "module",
    ecmaFeatures: {
      jsx: true,
    },
  },
  settings: {
    react: {
      version: "detect",
    },
  },
  extends: [
    "eslint:recommended",
  ],
  overrides: [
    {
      files: ["**/*.ts", "**/*.tsx"],
      rules: {
        // TS handles names, ESLint can false-positive here
        "no-undef": "off",
      },
    },
  ],
  ignorePatterns: [
    "dist/",
    "node_modules/",
    "**/*.d.ts",
    "_DEPRECATED_alpha_do_not_use/**",
  ],
};
