# CI/CD Security Hardening Plan

Based on recommendations from [Astral's open source security blog post](https://astral.sh/blog/open-source-security-at-astral), reviewed against this repo's GitHub Actions workflows.

## Issues & Fixes (priority order)

### 1. Pin all actions to commit SHAs

**Problem:** Every `uses:` reference across all workflows is pinned to a mutable tag (e.g., `actions/checkout@v4`). Tags can be moved by an attacker who compromises an action's repo. `taiki-e/install-action@nextest` is pinned to a branch name, not even a version.

**Affected files:** All workflow files in `.github/workflows/` and `.github/actions/use-sccache/action.yml`.

**Fix:** Replace every tag with the full commit SHA of the desired release. Add a comment with the version for readability:
```yaml
uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
```
Use [pinact](https://github.com/suzuki-shunsuke/pinact) to automate this.

---

### 2. Add restrictive default permissions to all workflows

**Problem:** Most workflows have no `permissions` block (inheriting broad defaults), and `benchmark.yml` grants `contents: write` + `pull-requests: write` at the workflow level, giving every job those permissions.

**Affected files:** `ci.yml`, `benchmark.yml`, `docs.yml`, `template.yml`, `build-website.yml`, `release.yml`.

**Fix:** Add `permissions: {}` at the workflow level in every file. Grant permissions per-job only where needed:
```yaml
permissions: {}

jobs:
  my_job:
    permissions:
      contents: write  # only if needed
```

---

### 3. Vendor the curled benchmark HTML script

**Problem:** `benchmark.yml` fetches and executes a TypeScript file from a mutable `master` branch at runtime:
```
curl -sSL https://raw.githubusercontent.com/benchmark-action/github-action-benchmark/master/src/default_index_html.ts
```
This is a runtime immutability gap — even if the action were SHA-pinned, this fetch is not.

**Affected files:** `benchmark.yml`.

**Fix:** Download `default_index_html.ts` once, commit it into the repo (e.g., `.github/scripts/default_index_html.ts`), and reference it locally. Alternatively, pin the URL to a specific commit SHA.

---

### 4. Add checksum verification for Maelstrom download

**Problem:** `ci.yml` downloads a binary tarball from GitHub releases without verifying its integrity:
```
curl -L -o ... https://github.com/jepsen-io/maelstrom/releases/download/v0.2.4/maelstrom.tar.bz2
```

**Affected files:** `ci.yml`.

**Fix:** Add a SHA256 checksum verification step after the download:
```yaml
echo "<EXPECTED_SHA256>  $MAELSTROM_DIR/maelstrom.tar.bz2" | sha256sum -c -
```
Compute the hash from a trusted download and hardcode it.

---

### ~~5. Replace `pull_request_target` trigger~~ (Won't fix)

**Reason:** The action's README explicitly recommends `pull_request_target` for public repos that accept fork PRs. Switching to `pull_request` would break the check for external contributors. The current usage is safe — the workflow only reads PR metadata and never checks out untrusted code.

---

### 6. Add deployment environment to release workflow

**Problem:** `release.yml` has no GitHub deployment environment, so there's no enforced approval gate or environment-specific secret isolation for releases.

**Affected files:** `release.yml` + GitHub repo settings.

**Fix:** Add `environment: release` to the release job:
```yaml
jobs:
  release_job:
    environment: release
```
Then in GitHub repo settings → Environments → create `release` with:
- Required reviewers (1-2 team members)
- Branch restriction to `main` only

---

### 7. Evaluate sccache / CI caching risk

**Problem:** The repo uses `sccache` via `mozilla-actions/sccache-action` in CI builds. Cache poisoning is a known GitHub Actions attack vector.

**Affected files:** `.github/actions/use-sccache/action.yml`, `ci.yml`.

**Fix:** This is lower priority since sccache is not used in the release workflow. Options:
- Accept the risk (CI cache doesn't affect release artifacts).
- Disable caching on `main` branch pushes if CI results on `main` are considered authoritative.
- No code change strictly required — conscious risk acceptance.
