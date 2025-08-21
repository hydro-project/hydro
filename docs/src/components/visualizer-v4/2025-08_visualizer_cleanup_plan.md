# Visualizer (v4) Cleanup Plan — August 2025

This document outlines a safe, measurable plan to clean up the visualizer code under `docs/src/components/visualizer-v4` and related pages such as `docs/src/pages/vis.js`, without breaking functionality.

## Status update — 2025-08-21

- Tests: 61 files; 282 passed, 2 skipped; 0 failed. SSR warnings from antd’s useLayoutEffect are expected and non-fatal.
- Smart-collapse guardrails: implemented with regression tests; fuzz/integration suites validate hyperEdge integrity during expand/collapse.
- Legacy renderers: FloatingEdge/HyperEdge removed from active `edgeTypes`; files retained as deprecated stubs. Internal hyperEdge logic preserved for collapsed routing only.
- ReactFlow bridge refactor: helpers extracted for containers/nodes/edges; parent-child `extent: 'parent'` and `parentMap` handling corrected; behavior parity confirmed by tests.
- Page wiring/components: `docs/src/pages/vis.js` simplified to use the visualizer-v4 barrel; effect dependencies corrected. `FileDropZone` split into `JSONFormatDocumentation` and `CompleteExampleDisplay` subcomponents.
- Config consolidation: `DEFAULT_RENDER_CONFIG` centralized in `shared/config.ts` with a typed wrapper in `render/config.ts`. Edge styling example updated to unified standard edges.
- ESLint/TypeScript hygiene (in-scope): targeted fixes and scoped disables where needed; no `lint` script currently defined in `docs/package.json`.
- Dev reports: refreshed under `docs/src/components/visualizer-v4/dev_reports/` via `dev_reports/refresh.sh`:
  - ts-prune.txt — refreshed
  - madge.json — refreshed
  - size-top40.txt — refreshed
  - knip.json — refreshed (knip may exit non-zero for findings; this is informational)

Quality gates snapshot
- Build: Vitest executed successfully within docs; no codegen/build regressions observed in tests.
- Lint/Typecheck: TypeScript compiled during tests; scoped ESLint clean in the modified areas.
- Tests: PASS (see counts above). Comprehensive fuzz tests remain skipped unless `ENABLE_FUZZ_TESTS=true`.

Optional low-risk next steps
- Add a docs-level npm `lint` script and (optionally) wire CI to run it.
- Extract inline styles from `FileDropZone` into a small styles module.
- Consider safe trims/splitting in `layout/ELKStateManager.ts` without behavior changes.
- Triage knip findings now that the report is fresh (whitelist expected dynamic/imported items).

## Status update — 2025-08-20

- Tests: green (npm test clean).
- Lint: clean (no warnings) within `docs/src/components/visualizer-v4`; a benign TS-ESTree version notice may appear during lint setup, but no file-level issues remain.
- Dev reports regenerated under `docs/src/components/visualizer-v4/dev_reports/`:
  - ts-prune.txt — refreshed.
  - madge.json — refreshed.
  - size-top40.txt — refreshed (top large files listed for prioritizing refactors).
  - knip.json — refreshed across `docs/` with reporter output written to the visualizer folder. Note: knip exits non-zero when it finds issues; several items are flagged for triage (e.g., unlisted plugins, local docusaurus plugin paths, optional coverage reporter). These are documented in the JSON for follow-up and do not affect runtime.
- Legacy rendering paths (floating edges/HyperEdge) have been removed from the active renderer, with smart-collapse guardrails and tests in place.

Snapshot (size-top40.txt, refreshed today)
- core/VisualizationState.ts — 1235 lines
- layout/ELKStateManager.ts — 923 lines
- core/EdgeStyleProcessor.ts — 846 lines
- core/JSONParser.ts — 720 lines
- bridges/ELKBridge.ts — 678 lines

Next steps
- Triage knip findings and either whitelist expected items (docs plugins, optional reporters) or address truly unused files/exports.
- Use size-top40 to guide file-splitting work (FlowGraph, VisualizationState, JSONParser) per Phases 4–5 below.
- Keep dev reports fresh after meaningful changes (see paths above) and ensure lint stays clean. A helper script is available at `docs/src/components/visualizer-v4/dev_reports/refresh.sh`.
- Legacy file deletion (safe, not referenced by runtime) — currently deferred: files are stubbed to preserve behavior while remaining out of active use.
  - `docs/src/components/visualizer-v4/render/FloatingEdge.tsx`
  - `docs/src/components/visualizer-v4/render/HyperEdge.tsx`
  - `docs/src/components/visualizer-v4/render/ReactFlowConverter.ts`

## Objectives

- Identify and remove dead code (files, exports, functions, constants).
- Ensure all configuration constants live in `shared/config.ts` (or clearly scoped submodules) and remove/consolidate duplicates.
- Drop unused constants from `shared/config.ts` and coalesce overlapping ones (e.g., edge/node style names, color palettes, typography).
- Identify complex/large files and propose refactors to simplify them.
- Maintain behavior parity: no regressions in rendering, interactivity, layout, or data parsing.

## Scope & change boundaries (hard constraint)

Only modify files in the following paths:
- `docs/src/pages/vis.js`
- `docs/src/pages/vis.module.css`
- `docs/src/components/visualizer-v4/**`

Out of scope for edits: any other files (e.g., `docs/package.json`, docusaurus configs, root configs). Tooling will be run locally without committing config changes, or changes will be isolated under `visualizer-v4` where possible.

## Guardrails (no breakage)

- Keep the docs site building and rendering `/vis` successfully.
- Add/extend quick tests around: JSON validation/parsing, VisualizationState operations, and FlowGraph rendering (smoke + basic interactions).
- Changes land in small, reviewable PRs with an easy revert path (you already have a baseline commit).

## Tools & Why

Dead code and unused exports/constants
- ts-prune: find unused TypeScript exports, including constants.
- knip: detect unused files/exports/dependencies across the workspace (supports monorepos well).
- eslint + @typescript-eslint + eslint-plugin-unused-imports: catch unused imports/vars during dev/CI.
- depcheck: highlight unused npm dependencies (especially under `docs/`).

Complexity, size, and structure
- ESLint rules: complexity, max-lines, max-lines-per-function, max-params (to set baselines and fail outliers).
- complexity-report (optional) or plato (legacy) for HTML reports; ESLint rules are usually sufficient.
- madge: visualize/catch circular deps, fan-in/out, orphaned modules.
- wc -l, cloc: quick line-count inventory to spot large files.

Reliability & build correctness
- tsc --noEmit with strict flags: enforce noUnusedLocals/Parameters and stricter typing for config.
- Docusaurus build (npm run build in `docs/`): ensures bundling still works.
- Vitest + @testing-library/react: smoke tests for FlowGraph and unit tests for core modules.

Developer ergonomics
- Prettier + ESLint integration, editor auto-fix (use existing repo config; do not modify configs outside the allowed scope).
- Optional: VS Code extensions for ESLint and TypeScript hero/import sorting.

## High-level Workflow (Phased)

Phase 0 — Safety Net (baseline checks within scope)
- Ensure `npm run build` succeeds in `docs/` (already green per latest run).
- Optional, if needed: add a small in-repo smoke harness under `visualizer-v4` (e.g., a minimal component and fixture data) to validate state transitions. Do not change tooling outside the allowed paths.
- Use existing ESLint/TypeScript config as-is; do not modify configs outside scope.

Phase 1 — Inventory & Metrics
- Run ts-prune and knip locally to produce reports of unused exports/files (no committed config changes).
- Run madge to generate a dependency graph and highlight cycles (local report).
- Use existing ESLint to collect complexity/size insights (no config changes); if needed, annotate results in the plan or PR descriptions.
- Run quick line counts to list largest files and longest functions.

Phase 2 — Config consolidation
- Source-of-truth: `docs/src/components/visualizer-v4/shared/config.ts`.
- Actions (within allowed paths only):
  - Move scattered constants (e.g., `TYPOGRAPHY` in `pages/vis.js`) into `shared/config.ts` or a `shared/ui.ts` within `visualizer-v4`, and import from there.
  - Consolidate overlapping edge/node style keys and palette names; ensure one canonical enum/type per family.
  - Introduce types: `RenderStyleConfig`, `PaletteName`, `EdgeStyleName`, `NodeStyleName`, etc., with `as const` where helpful.
  - Remove constants that are unused (confirmed by ts-prune/knip and grep) and update call sites.
  - Optional: add a minimal inline validation helper under `shared/` (TypeScript type guards) instead of external test frameworks.

Phase 3 — Dead code removal
- Remove unused exports/files surfaced by ts-prune/knip in small batches (only within allowed paths):
  - Delete unused components/helpers.
  - Fold simple one-off helpers back into modules if they’re only used once.
  - Build the docs site after each batch to ensure `/vis` still works.

Phase 4 — Simplify large/complex files
- Primary candidates (expected based on current structure):
  - `render/FlowGraph.tsx`: rendering + layout orchestration + event wiring.
  - `core/VisualizationState.ts`: state mutations, selectors, collection mgmt.
  - `core/JSONParser.ts`: validation + parsing + render-config creation.
- Refactor direction:
  - Extract `LayoutRunner` (ELK orchestration, fitView/refreshLayout) from FlowGraph.
  - Extract `EventBridge` (node click, label toggle, container expand/collapse) from FlowGraph into a hook (`useGraphEvents`) or a utility.
  - Split `JSONParser` into `schema/validation` and `parse/transform`; keep `createRenderConfig` focused.
  - In `VisualizationState`, separate immutable selectors from mutating methods; consider a small adapter layer for ReactFlow-specific data shapes.
  - Add file-level docstrings and module contracts; target max-lines and max-function-lines thresholds (no ESLint config changes outside scope; use comments/notes where necessary).

Phase 5 — Polish (within scope)
- Do not modify global CI or configs. Prefer documenting any remaining known issues and follow-ups.
- Keep code comments and module docs up to date.

## Concrete Commands (reference)

Note: Run these in `docs/` unless otherwise stated.

Inventory & dead code
- ts-prune
  - npx ts-prune --ignore "**/*.stories.ts*" > ../target/ts-prune.txt
- knip (monorepo-aware)
  - npx knip --workspace --production --reporter json > ../target/knip.json
- depcheck
  - npx depcheck

Structure & cycles
- madge
  - npx madge src --extensions ts,tsx --circular --warning --json > ../target/madge.json

Complexity & size
- ESLint (with rules enabled)
  - npx eslint "src/**/*.{ts,tsx,js,jsx}"
- Quick sizes
  - find src -name "*.ts*" -print0 | xargs -0 wc -l | sort -nr | head -n 30

Build & tests
- Build Docusaurus site
  - npm run build
- Vitest (to be added)
  - npx vitest run

## File/Module Targets (initial hypothesis)

- Large/complex:
  - `render/FlowGraph.tsx` — candidate for splitting into layout, events, view.
  - `core/VisualizationState.ts` — candidate for separating selectors vs mutations.
  - `core/JSONParser.ts` — separate validation/schema from parsing/transform.
- Config surface:
  - `shared/config.ts` — consolidate all render/style constants; add UI constants (typography, spacings, z-indexes) and palettes; remove unused keys.
  - Consider small submodules if `config.ts` becomes too large: `shared/ui.ts`, `shared/styles.ts`, `shared/palettes.ts` with a barrel `shared/index.ts`.

## Acceptance Criteria

- All constants used by the visualizer come from `shared/config.ts` (or its submodules) and have types.
- No reported unused exports from ts-prune/knip (ignore lists documented if needed).
- No unused imports/vars (ESLint clean).
- No circular dependencies (madge clean) or documented waivers.
- Complexity and size thresholds are met:
  - complexity <= 12 per function (configurable),
  - max-lines-per-function <= 120 (exceptions documented),
  - max-lines per file <= 600 for React components (or split).
- Docs site builds and `/vis` renders; smoke tests pass.
- Only files within the allowed paths were modified: `pages/vis.js`, `pages/vis.module.css`, and `components/visualizer-v4/**`.

## Rollback Strategy

- Small PRs; each phase/step re-runnable.
- Keep a revert-friendly commit order (delete only after passing tests/build).
- Maintain ignore lists (knip/ts-prune) in repo for transparency when necessary.

## Risks & Mitigations

- False positives in dead-code tools:
  - Mitigate with combined signals (ts-prune + knip + grep + runtime usage), and by whitelisting dynamic imports/exports.
- Visual regressions after config consolidation:
  - Mitigate with screenshots and a tiny image-diff step (optional) and manual smoke on `/vis`.
- Over-consolidation of config causing coupled modules:
  - Keep `config.ts` typed and layered; prefer small submodules and a barrel.

## Suggested PR Sequence

1) Add tests + lint/tsconfig tightening (warns allowed initially), no behavior change.
2) Inventory PR: add tool configs and commit reports under `target/` to baseline.
3) Config consolidation PR: move constants and type the config; update call sites.
4) Dead code removal PRs (split by folder: core/render/components).
5) File simplification PRs (start with FlowGraph, then JSONParser, then VisualizationState).
6) Turn ESLint/TS rules to errors; clean reports should be near zero.

## Requirements Coverage

- Dead code: Addressed via ts-prune/knip/depcheck + deletions (Phases 1 & 3).
- Constants in `shared/config.ts`: Phase 2 consolidates and types; unused removed.
- Drop unused/consolidate constants: Phase 2 using tool outputs and grep.
- Identify big/complex files and plan to simplify: Phases 1 & 4, with concrete extraction plan.
- No breakage: Guardrails, tests, and build checks throughout.

---

Contact: Maintainers of `visualizer-v4`. Execute phases incrementally; each PR should keep `/vis` usable at all times.
