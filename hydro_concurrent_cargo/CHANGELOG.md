

## v0.1.0-alpha.0 (2026-07-14)

### Chore

 - <csr-id-b4da85e789d7c97d183eef3f3d7915b5e7bc24f8/> prepare for release

### New Features

 - <csr-id-c128c2293c7b5c780e1c892d5a44505781b5a211/> parallel compilation with per-job target dirs and shared artifact symlinks [ci-full]
   Each compilation job gets its own `--target-dir` (under
   `{target}/jobs/{name}`) to
   avoid cargo's global artifact-dir lock, while sharing compiled artifacts
   via symlinks
   to `.fingerprint`, `build`, and `deps` in the shared target dir.
   
   A prebuild step compiles the dylib crate (or --lib for non-dylib) into
   the shared
   target dir before the parallel final builds start, ensuring all
   dependencies are ready.
   
   Also adds feature forwarding to the generated dylib crate's Cargo.toml.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 17 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2975](https://github.com/hydro-project/hydro/issues/2975)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2975](https://github.com/hydro-project/hydro/issues/2975)**
    - Parallel compilation with per-job target dirs and shared artifact symlinks [ci-full] ([`c128c22`](https://github.com/hydro-project/hydro/commit/c128c2293c7b5c780e1c892d5a44505781b5a211))
 * **Uncategorized**
    - Prepare for release ([`b4da85e`](https://github.com/hydro-project/hydro/commit/b4da85e789d7c97d183eef3f3d7915b5e7bc24f8))
</details>

