#!/bin/bash
set -euo pipefail

# Build, deploy, and test the KVS ECS stack.
#
# Usage:
#   ./deploy.sh          # build + deploy + test
#   ./deploy.sh --skip-test   # build + deploy only
#   ./deploy.sh --destroy     # tear down the stack

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

SKIP_TEST=false
DESTROY=false

for arg in "$@"; do
    case $arg in
        --skip-test) SKIP_TEST=true ;;
        --destroy) DESTROY=true ;;
        *) echo "Unknown arg: $arg"; exit 1 ;;
    esac
done

if [ "$DESTROY" = true ]; then
    echo "=== Destroying stack ==="
    cd "$SCRIPT_DIR"
    npm install --silent
    npx cdk destroy --force
    exit 0
fi

echo "=== Step 1: Build binaries ==="
cd "$WORKSPACE_DIR"
bash "$SCRIPT_DIR/build.sh"

echo ""
echo "=== Step 2: Deploy CDK stack ==="
cd "$SCRIPT_DIR"
npm install --silent
npx cdk deploy --require-approval never

if [ "$SKIP_TEST" = true ]; then
    echo ""
    echo "=== Deploy complete (tests skipped) ==="
    exit 0
fi

echo ""
echo "=== Step 3: Run integration tests ==="
cd "$WORKSPACE_DIR"
cargo test -p kvs --test ecs_integration -- --nocapture

echo ""
echo "=== All done ==="
