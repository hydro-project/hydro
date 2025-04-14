#!/usr/bin/env bash
set -euo pipefail

HELP="Usage: $0 [TARGET]...
Run pre-check tests for the given targets.

  --all         Run all tests
  --dfir        Run DFIR tests
  --hydro       Run Hydro tests
  --hydro-cli   Run tests for the Hydro CLI python interface
  --help        Display this help message
"

TEST_DFIR=false
TEST_HYDRO=false
TEST_HYDRO_CLI=false
TEST_ALL=false

while (( $# )) do
    case $1 in
        --dfir)
            TEST_DFIR=true
        ;;
        --hydro)
            TEST_HYDRO=true
        ;;
        --hydro-cli)
            TEST_HYDRO_CLI=true
        ;;
        --all)
            TEST_DFIR=true
            TEST_HYDRO=true
            TEST_HYDRO_CLI=true
            TEST_ALL=true
        ;;
        --help)
            echo "$HELP"
            exit 0
        ;;
        *)
            echo "$0: Unknown option: $1
Try '$0 --help' for more information.
"
            exit 1
        ;;
    esac
    shift
done

TARGETS=""
if [ "$TEST_DFIR" = true ]; then
    TARGETS="$TARGETS -p dfir_lang -p dfir_rs -p dfir_macro"
fi
if [ "$TEST_HYDRO" = true ]; then
    TARGETS="$TARGETS -p hydro_lang -p hydro_std -p hydro_test -p hydro_test_local -p hydro_test_local_macro -p hydro_deploy -p hydro_deploy_integration"
fi
if [ "$TEST_HYDRO_CLI" = true ]; then
    TARGETS="$TARGETS -p hydro_cli"
fi

if [ "$TEST_ALL" = true ]; then
    TARGETS="--all-targets"
elif [ "" = "$TARGETS" ]; then
    echo "$0: No targets specified.
Try '$0 --help' for more information.
"
    exit 2
fi

# Run the tests, echoing the commands as they are run
set -x

cargo +nightly fmt --all
cargo clippy $TARGETS --features dfir_rs/python -- -D warnings
[ "$TEST_ALL" = false ] || cargo check --all-targets --no-default-features

INSTA_FORCE_PASS=1 INSTA_UPDATE=always TRYBUILD=overwrite cargo test $TARGETS --no-fail-fast --features dfir_rs/python
cargo test $TARGETS --doc

[ "$TEST_DFIR" = false ] || CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test -p dfir_rs --target wasm32-unknown-unknown --tests --no-fail-fast

# Test that docs build.
RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo +nightly doc --no-deps --all-features
