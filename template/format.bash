#!/usr/bin/env bash
set -euxo pipefail
cd "$(dirname "$0")"

for MANIFEST in */Cargo.toml; do
    BAK="$(dirname "$MANIFEST")/Cargo.toml.bak"
    cp "$MANIFEST" "$BAK"
    # `cargo fmt` actually only needs the `edition` field, make dummy `Cargo.toml` without placeholders.
    sed -i -n '/\[package\]\|name\b\|edition\b/p' "$MANIFEST"
    cargo +nightly fmt --all --manifest-path "$MANIFEST"
    mv -f "$BAK" "$MANIFEST"
done