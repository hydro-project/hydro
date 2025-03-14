set -e

wget -qO- https://github.com/llvm/llvm-project/releases/download/llvmorg-19.1.0/LLVM-19.1.0-Linux-X64.tar.xz | tar xJ

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

source "$HOME/.cargo/env"

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
rustup toolchain install

cd website_playground

DFIR_GENERATE_DOCS="1" RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" CC="$PWD/../LLVM-19.1.0-Linux-X64/bin/clang" wasm-pack build

cd ..

RUSTDOCFLAGS="--cfg docsrs --cfg stageleft_runtime" RUSTDOCFLAGS="-Dwarnings" cargo doc --no-deps --all-features

cp -r target/doc docs/static/rustdoc

cd docs

npm ci

LOAD_PLAYGROUND=1 npm run build
