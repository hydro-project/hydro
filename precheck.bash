#!/usr/bin/env bash
set -euo pipefail

HELP="Usage: $0 [TARGET]...
Run pre-check tests for the given targets.

  --all         Run all tests
  --dfir        Run DFIR tests
  --hydro       Run Hydro tests
  --hydro-cli   Run tests for the Hydro CLI python interface
  --help        Display this help message"

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
Try '$0 --help' for more information."
            exit 1
        ;;
    esac
    shift
done

TARGETS_TEST=""
TARGETS_DOCS=""
FEATURES=""
if [ "$TEST_DFIR" = true ]; then
    TARGETS_TEST="$TARGETS_TEST -p dfir_lang -p dfir_rs -p dfir_macro"
    TARGETS_DOCS="$TARGETS_DOCS -p dfir_lang -p dfir_rs -p dfir_macro"
    FEATURES="$FEATURES --features dfir_rs/python"
fi
if [ "$TEST_HYDRO" = true ]; then
    TARGETS_TEST="$TARGETS_TEST -p hydro_lang -p hydro_std -p hydro_test -p hydro_test_local -p hydro_test_local_macro -p hydro_deploy -p hydro_deploy_integration"
    TARGETS_DOCS="$TARGETS_DOCS -p hydro_lang -p hydro_std -p hydro_test -p hydro_test_local -p hydro_test_local_macro -p hydro_deploy -p hydro_deploy_integration"
fi
if [ "$TEST_HYDRO_CLI" = true ]; then
    TARGETS_TEST="$TARGETS_TEST -p hydro_cli -p hydro_cli_examples"
    TARGETS_DOCS="$TARGETS_DOCS -p hydro_cli"
fi

if [ "$TEST_ALL" = true ]; then
    TARGETS_TEST="--workspace"
    TARGETS_DOCS="--workspace"
elif [ "" = "$TARGETS_TEST" ]; then
    echo "$0: No targets specified.
Try '$0 --help' for more information."
    exit 2
fi

# Run the tests, echoing the commands as they are run
set -x

cargo +nightly fmt --all
cargo clippy $TARGETS_TEST --all-targets $FEATURES -- -D warnings
[ "$TEST_ALL" = false ] || cargo check --all-targets --no-default-features

# `--all-targets` is everything except `--doc`: https://github.com/rust-lang/cargo/issues/6669.
INSTA_FORCE_PASS=1 INSTA_UPDATE=always TRYBUILD=overwrite cargo test $TARGETS_TEST --all-targets --no-fail-fast $FEATURES
cargo test $TARGETS_DOCS --doc

[ "$TEST_DFIR" = false ] || CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test -p dfir_rs --target wasm32-unknown-unknown --tests --no-fail-fast

# Test that docs build.
RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo +nightly doc --no-deps --all-features
