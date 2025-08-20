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
        // TS and code generation can result in unused symbols; disable these checks
        'no-unused-vars': 'off',
        'no-empty': 'off',
        'no-case-declarations': 'off',
        // Allow function declarations in blocks for recursive validators
        'no-inner-declarations': 'off',
        // TS handles name resolution
        'no-undef': 'off',
      },
    },
    {
      files: ["**/*.js"],
      rules: {
        // Scripts and generated code may have unused variables
        'no-unused-vars': 'off',
      },
    }
  ],
  ignorePatterns: [
    "dist/",
    "node_modules/",
    "**/*.d.ts",
    "_DEPRECATED_alpha_do_not_use/**",
  ],
};
