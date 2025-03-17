#!/usr/bin/env bash
set -euxo pipefail

FULL=true
if [ 0 -ne $# ]; then
    if [ "--quick" = "$1" ]; then
        FULL=false
    else
        echo "Unknown argument: `$1`"
        exit 1
    fi
fi

cargo +nightly fmt --all
cargo clippy --all-targets --features python -- -D warnings
[ "$FULL" = false ] || cargo check --all-targets --no-default-features

INSTA_FORCE_PASS=1 INSTA_UPDATE=always TRYBUILD=overwrite cargo test --all-targets --no-fail-fast --features python
cargo test --doc
[ "$FULL" = false ] || RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo doc --no-deps --all-features

[ "$FULL" = false ] || CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test -p dfir_rs --target wasm32-unknown-unknown --tests --no-fail-fast
