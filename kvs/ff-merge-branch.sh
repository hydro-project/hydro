#!/bin/bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <branch-name>"
    echo "Example: $0 sandbox-d5f3a907-28d8-4243-aad0-92dd0a4d3036"
    exit 1
fi

BRANCH="$1"
WORKSPACE_SRC="$(cd "$(dirname "$0")/.." && pwd)"

REPOS=(
    HydroProjectDemoApp
    HydroProjectDemoAppCDK
    HydroProjectDemoAppImageBuild
    HydroProjectDemoAppTests
    HydroProjectHydro
    HydroProjectStageleft
)

for repo in "${REPOS[@]}"; do
    dir="${WORKSPACE_SRC}/${repo}"
    if [[ ! -d "${dir}/.git" ]]; then
        echo "SKIP  ${repo} — not a git repo"
        continue
    fi

    current=$(git -C "$dir" rev-parse --abbrev-ref HEAD)
    echo "───── ${repo} (on ${current}) ─────"

    if ! git -C "$dir" rev-parse --verify "$BRANCH" &>/dev/null; then
        echo "  SKIP — branch ${BRANCH} not found locally"
        continue
    fi

    # Check if there are actual changes (non-empty commits) between current and the branch
    new_commits=$(git -C "$dir" log --oneline "${current}..${BRANCH}" --diff-filter=ACDMRT -- 2>/dev/null | wc -l)
    if [[ "$new_commits" -eq 0 ]]; then
        echo "  SKIP — no new changes on ${BRANCH}"
        continue
    fi

    if git -C "$dir" merge --ff-only "$BRANCH" 2>/dev/null; then
        echo "  OK — fast-forward merged ${BRANCH} into ${current} (${new_commits} commit(s))"
    else
        echo "  FAIL — cannot fast-forward merge ${BRANCH} into ${current}"
    fi
done
