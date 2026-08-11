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
    --maelstrom   Run Maelstrom tests (requires --hydro)
  --website       Run Website tests
  --help          Display this help message
"

TEST_DFIR=false
TEST_HYDRO=false
TEST_DOCKER=false
TEST_ECS=false
TEST_MAELSTROM=false
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
        --maelstrom)
            TEST_MAELSTROM=true
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
            TEST_MAELSTROM=true
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

# If `--docker`/`--ecs`/`--maelstrom`, then ensure `--hydro` was also included.
if ( [ "$TEST_DOCKER" = true ] || [ "$TEST_ECS" = true ] || [ "$TEST_MAELSTROM" = true ] ) && [ "$TEST_HYDRO" = false ]; then
    echo "$0: --hydro is required for any of --docker, --ecs, --maelstrom.
Try '$0 --help' for more information.
"
    exit 3
fi
# If `--wasm`, ensure `--dfir` was also included.
if [ "$TEST_WASM" = true ] && [ "$TEST_DFIR" = false ]; then
    echo "$0: --dfir is required for --wasm.
Try '$0 --help' for more information.
"
    exit 4
fi

TARGETS=""
FEATURES=""
if [ "$TEST_DFIR" = true ]; then
    TARGETS="$TARGETS -p dfir_lang -p dfir_pipes -p dfir_rs -p dfir_macro -p lattices -p variadics"
fi
if [ "$TEST_HYDRO" = true ]; then
    TARGETS="$TARGETS -p hydro_lang -p hydro_std -p hydro_test -p hydro_test_embedded -p hydro_deploy -p hydro_deploy_integration"
    FEATURES="$FEATURES --features deploy,sim"

    if [ "$TEST_DOCKER" = true ]; then
        FEATURES="$FEATURES --features docker"
    fi
    if [ "$TEST_ECS" = true ]; then
        FEATURES="$FEATURES --features ecs"
    fi
    if [ "$TEST_MAELSTROM" = true ]; then
        export MAELSTROM_PATH="${MAELSTROM_PATH:="$HOME/maelstrom/maelstrom"}"
        export RUSTFLAGS='--cfg=maelstrom_available'
        # Check if `MAELSTROM_PATH` exists as an executable.
        if [ ! -x "$MAELSTROM_PATH" ]; then
            echo "$0: Maelstrom executable not found at \$MAELSTROM_PATH: $MAELSTROM_PATH.
Download Maelstrom from https://github.com/jepsen-io/maelstrom/releases and
extract it to \$HOME, or set \$MAELSTROM_PATH, and make sure it is executable."
            exit 5
        fi
        # Check if `java` is installed.
        if ! command -v java &> /dev/null; then
            echo "$0: Java could not be found. Please install Java to run Maelstrom tests."
            exit 6
        fi
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

./template/generate_prompts.py
cargo +nightly fmt --all

cargo clippy $TARGETS --keep-going --all-targets --no-default-features -- -D warnings
cargo clippy $TARGETS --keep-going --all-targets --all-features -- -D warnings
cargo clippy $TARGETS --keep-going --all-targets $FEATURES -- -D warnings

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
