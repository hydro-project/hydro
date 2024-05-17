#!/bin/bash
set -euxo pipefail

cargo +nightly fmt --all
cargo clippy --all-targets --features python -- -D warnings

INSTA_FORCE_PASS=1 INSTA_UPDATE=always TRYBUILD=overwrite cargo test --all-targets --no-fail-fast --features python
cargo test --doc
