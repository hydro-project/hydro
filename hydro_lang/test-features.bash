#!/usr/bin/env bash
set -euxo pipefail

export RUSTFLAGS="-Dwarnings"

cargo test -p hydro_lang --no-default-features
cargo test -p hydro_lang --no-default-features --features build
cargo test -p hydro_lang --no-default-features --features trybuild
cargo test -p hydro_lang --no-default-features --features deploy
cargo test -p hydro_lang --no-default-features --features telemetry
cargo test -p hydro_lang --no-default-features --features sim
cargo test -p hydro_lang --no-default-features --features viz
cargo test -p hydro_lang --all-features
