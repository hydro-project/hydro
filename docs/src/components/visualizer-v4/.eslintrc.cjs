/* Hardened ESLint config for visualizer-v4 TypeScript/React code */
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
    "plugin:@typescript-eslint/recommended",
    "plugin:react/recommended",
    "plugin:react-hooks/recommended",
  ],
  overrides: [
    {
      files: ["**/*.ts", "**/*.tsx"],
      rules: {
        // Prefer TypeScript-aware unused vars rule; allow leading underscore to mark intent
        "no-unused-vars": "off",
        "@typescript-eslint/no-unused-vars": [
          "warn",
          { argsIgnorePattern: "^_", varsIgnorePattern: "^_", ignoreRestSiblings: true }
        ],
        // Allow empty blocks in some generated/test utilities
        "no-empty": "off",
        "no-case-declarations": "off",
        // TS handles name resolution
        "no-undef": "off",
        // Safe, low-noise improvements
        "eqeqeq": ["warn", "smart"],
        "no-debugger": "warn",
        "no-console": ["warn", { allow: ["warn", "error"] }],
        "prefer-const": "warn",
        // React/JSX tweaks for TS projects
        "react/react-in-jsx-scope": "off",
        "react/prop-types": "off",
        "react/jsx-key": "warn",
      },
    },
    {
      files: ["**/*.js"],
      rules: {
        // Scripts and generated code may have unused variables
        "no-unused-vars": "warn",
        "eqeqeq": ["warn", "smart"],
        "no-debugger": "warn",
        "no-console": ["warn", { allow: ["warn", "error"] }],
        "prefer-const": "warn",
      },
    }
  ],
  ignorePatterns: [
    "dist/",
    "node_modules/",
    "**/*.d.ts",
    "_DEPRECATED_alpha_do_not_use/**",
  "__tests__/**",
  ],
};
