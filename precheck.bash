#!/usr/bin/env bash
set -euo pipefail

HELP="Usage: $0 [TARGET]...
Run pre-check tests for the given targets.

  --all         Run all tests
  --dfir        Run DFIR tests
  --hydro       Run Hydro tests
  --help        Display this help message
"

TEST_DFIR=false
TEST_HYDRO=false
TEST_WEBSITE=false
TEST_ALL=false

while (( $# )); do
    case $1 in
        --dfir)
            TEST_DFIR=true
        ;;
        --hydro)
            TEST_HYDRO=true
        ;;
        --website)
            TEST_WEBSITE=true
        ;;
        --all)
            TEST_DFIR=true
            TEST_HYDRO=true
            TEST_WEBSITE=true
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
FEATURES=""
if [ "$TEST_DFIR" = true ]; then
    TARGETS="$TARGETS -p dfir_lang -p dfir_rs -p dfir_macro"
    FEATURES="$FEATURES --features dfir_rs/python"
fi
if [ "$TEST_HYDRO" = true ]; then
    TARGETS="$TARGETS -p hydro_lang -p hydro_std -p hydro_test -p hydro_deploy -p hydro_deploy_integration"
fi
if [ "$TEST_WEBSITE" = true ]; then
    TARGETS="$TARGETS -p website_playground"
fi

if [ "$TEST_ALL" = true ]; then
    TARGETS="--workspace"
elif [ "" = "$TARGETS" ]; then
    echo "$0: No targets specified.
Try '$0 --help' for more information.
"
    exit 2
fi

# Run the tests, echoing the commands as they are run
set -x

cargo +nightly fmt --all
cargo clippy $TARGETS --all-targets $FEATURES -- -D warnings
[ "$TEST_ALL" = false ] || cargo check --all-targets --no-default-features

# `--all-targets` is everything except `--doc`: https://github.com/rust-lang/cargo/issues/6669.
INSTA_FORCE_PASS=1 INSTA_UPDATE=always TRYBUILD=overwrite cargo test $TARGETS --all-targets --no-fail-fast $FEATURES
cargo test $TARGETS --doc

# Test website_playground wasm build.
if [ "$TEST_WEBSITE" = true ]; then
    pushd website_playground
    RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" wasm-pack build
    popd
fi

[ "$TEST_DFIR" = false ] || CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test -p dfir_rs --target wasm32-unknown-unknown --tests --no-fail-fast

# Test that docs build.
RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo +nightly doc --no-deps --all-features
