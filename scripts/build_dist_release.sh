#!/bin/bash

set -e

if [ "$#" -ne 2 ]; then
    echo "Usage: build_dist_release.sh <target OS> <target architecture>"
    exit 1
fi

TARGET_OS=$1
TARGET_ARCH=$2

case $TARGET_ARCH in
  arm64)
    RUST_TARGET_ARCH=aarch64
  ;;
  amd64)
    RUST_TARGET_ARCH=x86_64
    ;;
  *)
    echo "Unsupported architecture '${TARGET_ARCH}'"
    exit 1
    ;;
esac

case $TARGET_OS in
  linux)
    RUST_TARGET_OS=unknown-linux
    RUST_TARGET_LIBS=gnu
  ;;
  macos)
    RUST_TARGET_OS=apple
    RUST_TARGET_LIBS=darwin
  ;;
  *)
    echo "Unsupported OS '${TARGET_OS}'"
    exit 1
  ;;
esac

RUST_TARGET="${RUST_TARGET_ARCH}-${RUST_TARGET_OS}-${RUST_TARGET_LIBS}"

echo "Attempting to (cross-)compile to target '${RUST_TARGET}'"

rustup target add ${RUST_TARGET}

if [ "${TARGET_OS}" == "linux" ]
then
  export RUSTFLAGS="-C linker=${RUST_TARGET_ARCH}-linux-gnu-gcc"
fi

# The CARGO_NET_GIT_FETCH_WITH_CLI="true" environment variable is a Workaround to an issue similar
# to the one encountered by pytorch in https://github.com/pytorch/pytorch/issues/82174

CARGO_NET_GIT_FETCH_WITH_CLI="true" cargo build --release --all-targets --workspace --exclude hydro_deploy --exclude hydroflow_plus_test --exclude hydroflow_plus_test_macro --target ${RUST_TARGET}
