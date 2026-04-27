# JJ → GitHub PR Sync Tool

## Overview

This tool provides a minimal, predictable way to map a Jujutsu (jj) change graph onto GitHub Pull Requests (PRs).

The core philosophy is:

- **JJ owns structure (DAG of changes)**
- **GitHub owns presentation (branch/bookmark names, PR UI, title, description, review state)**
- **This tool synchronizes the two with no external hidden state beyond JJ descriptions and GitHub PR metadata**

---

## Goals

### Primary Goals

- Each PR corresponds to a **jj bookmark**
- Each PR may contain **multiple commits**
- Not all bookmarks should be PRs
- No setup required for reviewers (GitHub UI only)
- Simple, predictable CLI
- Must support a DAG of PRs, not just a linear stack
- Maintains a linear GitHub target branch history (assume rebase or squash merges)
- Does not require a linear GitHub target branch history to work
- PRs IDs may be created in any order, and merged in any order that respects the PR DAG ordering

### Secondary Goals (Nice-to-Have)

- Automatically mark PRs as ready when parent PRs are merged
- Display a JJ-style graph annotated with PR state
- Optionally embed graph visualization in PRs
- Handle agent-driven workflows (e.g., Agents or users editing GitHub PR metadata, Copilot suggested changes commits on PRs)

---

## Non-Goals (v1)

- Full workflow management (no rebase commands, no stack editing)
- Complex state tracking or caching
- Automatic handling/resolution of invalid state/errors
- Perfect handling of all edge cases
- Multi-forge support (GitHub only)

---

## Core Concepts

### Bookmark = PR Unit

Each PR corresponds 1:1 with a jj bookmark.

Example:

```
pr/feature-a
pr/feature-b
```

---

### PR Identification

The changes within a PR will be updated to contain the PR number as metadata in each description:

```
PR: #1234
```

This provides:
- Stable identity
- Recovery after rebases
- No external state file required

---

### PR Eligibility

A bookmark is a PR if it is the source of a PR on github.

If this proves to be too noisy, we may enforce a particular bookmark name format.

Approach:
- v1: User is expected to manually create the PR once the bookmark is tracked and pushed.
- Later: Provide a sub-command to create a PR with a specified branch name (or generated branch name).

---

### DAG Structure

PR DAG is derived from the jj graph:

- For a bookmark, find all nearest ancestors are from a different PR (or the trunk branch main)
- Those become the **parent PRs**

---

## CLI Design

### Commands (v1)

#### `jj pr log`

Displays PRs as a graph:

```
@   feature-c   PR #125  (draft)
├─╮
│ ○ feature-a   PR #126  (ready)
│ │
○ │ feature-b   PR #124  (ready)
├─╯
○  main
```

Purpose:
- Primary visualization tool
- Helps understand DAG structure and status

---

#### `jj pr sync`

Reconciles local jj state with GitHub.

Responsibilities:
- Find all PR bookmarks (via `gh` cli) that also exist (and track the remote) locally.
- Push changed PR bookmarks
- Detect merged PRs
- Rebase children onto merged parents
- Update draft/ready status

Optional flags:

```
--dry-run
--verbose
```

#### `jj pr create`

Responsibilities:
- Creates a new bookmark if an existing bookmark is not specified
- Ensures the bookmark tracks the remote
- Finds all changes in the PR
- Updates the change description with `PR: #1234` meta
- Creates a deterministic title + description from the changes in the PR, or uses the user supplied title/description
- Creates a github PR

---

## State Architecture

### Source of Truth

- JJ graph → structure
- GitHub → PR metadata (title, description, branch/bookmark inclusion)
- JJ change description (`PR: #1234`) → PR membership for the change

### Bookmark PR membership

A bookmark is considered a PR if all of the following are true:
* The bookmark tracks a remote branch
* The remote branch it is tracking has the same name
* The remote branch it is tracking is the source (HEAD) of an open PR

### Change PR membership

A change is included in a PR if both of the following are true:
* The change has a line `PR: #1234` matching the PR id
* One of:
  * The bookmark for the PR is pointing at the change
  * At least one child of the change is a member of the same PR.

It is considered an invalid state to have the former (`PR: 1234` tag) without one of the later, and an error should be logged if this is encountered.

### PR parent PRs

We model PRs as forming a DAG, not just a simple stack, so it may have multiple parents.
A parent is either a different PR or `trunk()` (usually `main`).

A change is parent of a PR if it is not a member of the PR, and if any child change is a member of a PR.
A parent change should be a member of another "parent" PR, or `trunk()`. The child PR is in an invalid
state if a parent change is not one of these.

---

### Data Flow

1. Read local jj graph (`jj log --json`), extract PR numbers from descriptions
2. Query GitHub `gh` CLI for PR bookmarks, state (pending, ready, merged)
3. Compute list of changes to make
4. Apply changes (if not `--dry-run`)

1 and 2 may be done concurrently.

Note that there is a external race condition between reading from GitHub (step 2) and writing to GitHub (step 4). Fine for v1.

---

## Sync Algorithm

### Step 1: Discover Local State

- Parse jj graph
- Identify PR bookmarks
- Extract:
  - bookmark name
  - commit range
  - PR number (if exists)

---

### Step 2: Load GitHub State

For each PR:
- PR number
- status (open, merged)
- source (head) branch
- destination (base) branch

---

### Step 3.1: Handle Merged PRs

PRs are expected to merged via the GitHub website, using squash or rebase.
PR branches are expected to be deleted by GitHub, so we don't delete them ourselves (for v1).

For each merged PR, if the merged change is different from the local branch's changes, compute changes: forget the old branch bookmark & abandon the changes

---

### Step 3.2: Draft / Ready State

Rule:

- Draft if parent PR is not merged
- Ready if all ancestors are merged

Compute changes: change PR to ready or draft

---

### Step 3.3: Rebase Children

All PRs that are children of at least one merged PR must be rebased.
All PRs that are children of `trunk()` must be rebased when `trunk()` changes.

Each PR to be rebased should be rebased upon all its parents. However note
that if multiple of its parents are rebased to `trunk()`, then they combine
into a single `trunk()` parent.

```
jj rebase -s <child> -o <parent_commit> [-o <other_parent_commits>]
```

Note that due to the way `jj` works (specifically with `-s`),
only the direct children need to be updated, not the whole DAG.

---

## PR Metadata Handling

### Title & Description

- GitHub is the source of truth
- Tool does NOT overwrite

This allows:
- Manual edits in UI
- Agent-driven updates via MCP

---

### Philosophy (v1)

- No automatic recovery or correction of invalid states
- Tool should be transparent and predictable
- Errors should be logged clearly to the user, with enough context for manual resolution

---

### Missing PR Metadata

If `PR: #` is missing:
- Log error (as above)
- Skip bookmark

Also applies to failure to compute the start change of the PR.

### PR force-pushes

When a PR is force-pushed, or updated locally and remotely separately, the branch may end up in a divergent state.
This is considered an invalid state, and should be logged, but it is up to the user to manually resolve the issue.

---

### Idempotency

`jj pr sync` should be safe to run repeatedly, and result in no additional changes after the first run (if nothing else changes).

---

## Implementation Plan

### Phase 1 (v1)

- Parse `jj log --json`
- Identify PR bookmarks
- Extract PR numbers from descriptions (`PR: #123`)
- Implement `jj pr log` (graph + PR annotations)
- Implement `jj pr sync`:
  - push PR bookmarks only
  - load GitHub PR state
  - validate mapping (error if missing PR metadata)
  - update base branches (DAG-aware)

---

### Phase 2

- Draft/ready automation (based on parent PR merge state)
- Merge detection via GitHub `gh` cli
- Rebase children onto merged parents (`jj rebase -s`)
- Improve DAG handling (non-linear stacks)
- Optional graph export (e.g., mermaid)

---

### Phase 3

- Graph visualization improvements
- PR description should have link to the diff view with only PR's changes/commits selected
- PR comments (mermaid graphs)
- Better error handling

---

## Summary

This tool intentionally prioritizes:

- Simplicity, predictability
- Alignment with jj workflows
- Statelessness, using external sources of truth that are user-visible and manually fixable

By keeping state minimal and explicit, it avoids the complexity and fragility seen in more ambitious tools while still solving the core problem of managing multiple PRs.

