#!/bin/bash
set -euo pipefail

# Build the KVS binaries and prepare the Docker context for CDK deploy.
#
# Usage: ./build.sh
#
# This script:
# 1. Generates the hydro-manifest.json via ECS export
# 2. Builds the hydro trybuild binaries from the generated crate
# 3. Copies them into docker/bin/ for the Dockerfile

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KVS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKSPACE_DIR="$(cd "$KVS_DIR/.." && pwd)"

echo "=== Step 1: Generate hydro-manifest.json ==="
cargo run -p kvs --example kvs -- --mode export --output "$KVS_DIR/hydro-assets"

echo "=== Step 2: Read manifest ==="
MANIFEST="$KVS_DIR/hydro-assets/hydro-manifest.json"

# Extract build info from the manifest
read -r PROJECT_DIR TARGET_DIR FEATURES <<< "$(python3 -c "
import json
m = json.load(open('$MANIFEST'))
vals = list(m.get('processes', {}).values()) + list(m.get('clusters', {}).values())
v = vals[0]
print(v['build']['project_dir'], v['build']['target_dir'], ','.join(v['build']['features']))
")"

BIN_NAMES=$(python3 -c "
import json
m = json.load(open('$MANIFEST'))
bins = set()
for p in m.get('processes', {}).values():
    bins.add(p['build']['bin_name'])
for c in m.get('clusters', {}).values():
    bins.add(c['build']['bin_name'])
for b in sorted(bins):
    print(b)
")

echo "Project dir: $PROJECT_DIR"
echo "Target dir: $TARGET_DIR"
echo "Features: $FEATURES"
echo "Binaries:"
echo "$BIN_NAMES"

echo "=== Step 3: Build binaries from trybuild crate ==="
# The trybuild crate is generated outside the workspace, so we build
# from its directory directly using --manifest-path.
# The hydro binaries are generated as examples in the trybuild crate.
EXAMPLE_FLAGS=""
for bin in $BIN_NAMES; do
    EXAMPLE_FLAGS="$EXAMPLE_FLAGS --example $bin"
done

STAGELEFT_TRYBUILD_BUILD_STAGED=1 cargo build \
    --manifest-path "$PROJECT_DIR/Cargo.toml" \
    --features "$FEATURES" \
    --target-dir "$TARGET_DIR" \
    $EXAMPLE_FLAGS

echo "=== Step 4: Copy binaries to Docker context ==="
rm -rf "$SCRIPT_DIR/docker/bin"
mkdir -p "$SCRIPT_DIR/docker/bin"

for bin in $BIN_NAMES; do
    SRC="$TARGET_DIR/debug/examples/$bin"
    if [ -f "$SRC" ]; then
        cp "$SRC" "$SCRIPT_DIR/docker/bin/"
        echo "  Copied $bin ($(du -h "$SRC" | cut -f1))"
    else
        echo "  ERROR: $bin not found at $SRC" >&2
        exit 1
    fi
done

echo "=== Build complete ==="
echo "Docker context ready at $SCRIPT_DIR/docker/"
echo "Run 'cd $SCRIPT_DIR && npx cdk deploy' to deploy."
