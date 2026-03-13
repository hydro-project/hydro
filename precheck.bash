#!/usr/bin/env bash
set -euo pipefail

HELP="Usage: $0 [TARGET]...
Run pre-check tests for the given targets.

  --all           Run all tests
  --dfir          Run DFIR tests
    --wasm        Run WASM tests (requires --dfir)
  --hydro         Run Hydro tests
    --docker      Run Docker tests (requires --hydro)
    --ecs         Run ECS tests (requires --hydro)
  --website       Run Website tests
  --help          Display this help message
"

TEST_DFIR=false
TEST_HYDRO=false
TEST_DOCKER=false
TEST_ECS=false
TEST_WEBSITE=false
TEST_WASM=false
TEST_ALL=false

while (( $# )); do
    case $1 in
        --dfir)
            TEST_DFIR=true
        ;;
        --hydro)
            TEST_HYDRO=true
        ;;
        --docker)
            TEST_DOCKER=true
        ;;
        --ecs)
            TEST_ECS=true
        ;;
        --website)
            TEST_WEBSITE=true
        ;;
        --wasm)
            TEST_WASM=true
        ;;
        --all)
            TEST_DFIR=true
            TEST_HYDRO=true
            TEST_DOCKER=true
            TEST_ECS=true
            TEST_WEBSITE=true
            TEST_WASM=true
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

# If either `--docker` or `--ecs`, ensure `--hydro` was also included.
if ( [ "$TEST_DOCKER" = true ] || [ "$TEST_ECS" = true ] ) && [ "$TEST_HYDRO" = false ]; then
    echo "$0: --docker and --ecs require --hydro.
Try '$0 --help' for more information.
"
    exit 3
fi
# If `--wasm`, ensure `--dfir` was also included.
if [ "$TEST_WASM" = true ] && [ "$TEST_DFIR" = false ]; then
    echo "$0: --wasm requires --dfir.
Try '$0 --help' for more information.
"
    exit 4
fi

TARGETS=""
FEATURES=""
if [ "$TEST_DFIR" = true ]; then
    TARGETS="$TARGETS -p dfir_lang -p dfir_pipes -p dfir_rs -p dfir_macro"
fi
if [ "$TEST_HYDRO" = true ]; then
    TARGETS="$TARGETS -p hydro_lang -p hydro_std -p hydro_test -p hydro_deploy -p hydro_deploy_integration"
    FEATURES="$FEATURES --features deploy,sim"

    if [ "$TEST_DOCKER" = true ]; then
        FEATURES="$FEATURES --features docker"
    fi
    if [ "$TEST_ECS" = true ]; then
        FEATURES="$FEATURES --features ecs"
    fi
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
INSTA_FORCE_PASS=1 INSTA_UPDATE=always TRYBUILD=overwrite cargo nextest run $TARGETS --all-targets --no-fail-fast $FEATURES
cargo test $TARGETS --doc

# Test website_playground wasm build.
if [ "$TEST_WEBSITE" = true ]; then
    pushd website_playground
    rustup toolchain install nightly
    RUSTUP_TOOLCHAIN="nightly" RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" wasm-pack build
    popd
fi

if [ "$TEST_DFIR" = true ] && [ "$TEST_WASM" = true ]; then
    rustup toolchain install nightly
    RUSTUP_TOOLCHAIN="nightly" CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test -p dfir_rs --target wasm32-unknown-unknown --tests --no-fail-fast
fi

# Test that docs build.
RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo +nightly doc --no-deps --all-features
