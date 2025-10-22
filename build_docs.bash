set -e

ts() { date +%s; }
fmt() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }
step_start() { echo "[TIMING] $(fmt) START $1"; echo "[TIMING] $1_T0=$(ts)"; }
step_end() {
	local name="$1"; local t0_var="$2"; local t0
	# Extract last printed T0 for the step from the logs if available, else compute delta naively.
	# Since we don't store state across shells, we measure coarse-grained by printing end timestamps too.
	echo "[TIMING] $(fmt) END   $name";
}

echo "========================================="
echo "Step 1/7: Downloading LLVM..."
echo "========================================="
step_start "STEP1_LLVM"
wget -qO- https://github.com/llvm/llvm-project/releases/download/llvmorg-19.1.0/LLVM-19.1.0-Linux-X64.tar.xz | tar xJ
step_end "STEP1_LLVM"

echo "========================================="
echo "Step 2/7: Installing Rust..."
echo "========================================="
step_start "STEP2_RUST"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

source "$HOME/.cargo/env"
step_end "STEP2_RUST"

echo "========================================="
echo "Step 3/7: Installing wasm-pack and nightly toolchain..."
echo "========================================="
step_start "STEP3_WASM_PACK"
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
rustup toolchain install nightly
step_end "STEP3_WASM_PACK"

echo "========================================="
echo "Step 4/7: Building WebAssembly playground..."
echo "========================================="
step_start "STEP4_PLAYGROUND"
cd website_playground

RUSTUP_TOOLCHAIN="nightly" RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" CC="$PWD/../LLVM-19.1.0-Linux-X64/bin/clang" wasm-pack build

cd ..
step_end "STEP4_PLAYGROUND"

echo "========================================="
echo "Step 5/7: Building Rust documentation..."
echo "========================================="
step_start "STEP5_RUSTDOC"
RUSTUP_TOOLCHAIN="nightly" RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo doc --no-deps --all-features

cp -r target/doc docs/static/rustdoc
step_end "STEP5_RUSTDOC"

cd docs

echo "========================================="
echo "Step 6/7: Installing npm dependencies..."
echo "========================================="
step_start "STEP6_NPM_CI"
npm ci
step_end "STEP6_NPM_CI"

echo "========================================="
echo "Step 7/7: Building Docusaurus site..."
echo "Note: This step may take several minutes during minification (TerserPlugin)"
echo "Note: Set SKIP_MINIFY_ALL=1 to speed up builds by disabling minification"
echo "SKIP_MINIFY_ALL=${SKIP_MINIFY_ALL:-0} SKIP_MINIFY_HYDROSCOPE=${SKIP_MINIFY_HYDROSCOPE:-0} CI=${CI:-}"
echo "========================================="
step_start "STEP7_DOCUSAURUS"
LOAD_PLAYGROUND=1 SKIP_MINIFY_ALL="${SKIP_MINIFY_ALL:-0}" SKIP_MINIFY_HYDROSCOPE="${SKIP_MINIFY_HYDROSCOPE:-0}" npm run build
step_end "STEP7_DOCUSAURUS"

echo "========================================="
echo "Build complete!"
echo "========================================="
