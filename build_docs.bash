set -e

echo "========================================="
echo "Step 1/7: Downloading LLVM..."
echo "========================================="
wget -qO- https://github.com/llvm/llvm-project/releases/download/llvmorg-19.1.0/LLVM-19.1.0-Linux-X64.tar.xz | tar xJ

echo "========================================="
echo "Step 2/7: Installing Rust..."
echo "========================================="
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

source "$HOME/.cargo/env"

echo "========================================="
echo "Step 3/7: Installing wasm-pack and nightly toolchain..."
echo "========================================="
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
rustup toolchain install nightly

echo "========================================="
echo "Step 4/7: Building WebAssembly playground..."
echo "========================================="
cd website_playground

RUSTUP_TOOLCHAIN="nightly" RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" CC="$PWD/../LLVM-19.1.0-Linux-X64/bin/clang" wasm-pack build

cd ..

echo "========================================="
echo "Step 5/7: Building Rust documentation..."
echo "========================================="
# Set `SKIP_RUSTDOC=1` to use a prebuilt rustdoc tree already placed at `docs/static/rustdoc`
# instead of running `cargo doc` here. CI does this to reuse the rustdoc built by the
# `docs` job (see `.github/workflows/docs.yml`) rather than building it a second time.
if [ "${SKIP_RUSTDOC:-0}" = "1" ]; then
  if [ ! -f docs/static/rustdoc/hydro_lang/index.html ]; then
    echo "SKIP_RUSTDOC=1 but docs/static/rustdoc does not contain a rustdoc build" >&2
    exit 1
  fi
  echo "SKIP_RUSTDOC=1: using prebuilt rustdoc in docs/static/rustdoc"
else
  RUSTUP_TOOLCHAIN="nightly" RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo doc --no-deps --all-features

  cp -r target/doc docs/static/rustdoc
fi

cd docs

echo "========================================="
echo "Step 6/7: Installing npm dependencies..."
echo "========================================="
npm ci

echo "========================================="
echo "Step 7/7: Building Docusaurus site..."
echo "========================================="
LOAD_PLAYGROUND=1 npm run build

echo "========================================="
echo "Build complete!"
echo "========================================="
