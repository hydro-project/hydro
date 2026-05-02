#!/bin/bash
set -euo pipefail

if [[ $# -eq 0 ]]; then
    echo "Usage: $0 <git-args...>"
    echo "Example: $0 status"
    echo "Example: $0 log --oneline -5"
    exit 1
fi

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
    echo "───── ${repo} ─────"
    git -C "$dir" "$@" || true
done
