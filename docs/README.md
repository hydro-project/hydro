# Hydro Docs
This website is built using [Docusaurus 2](https://docusaurus.io/), a modern static website generator.

You'll need Node installed to build the website. First, install the necessary dependencies:

```bash
$ npm ci
```

Finally, you can run the website locally:

```bash
$ npm run start
```

## Building the DFIR docs
`dfir/syntax/surface_ops_gen.md` is generated during the Rust build. You will need to run this command to first generate or update it:
```bash
DFIR_GENERATE_DOCS="1" cargo build -p dfir_macro -p dfir_lang
```

## Building the Playground
By default, the DFIR / Datalog playgrounds are not loaded when launching the website. To build the playground, you'll need to follow a couple additional steps. This requires Rust and [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```bash
$ rustup target add wasm32-unknown-unknown
$ cargo install wasm-pack
$ cd ../website_playground
$ CARGO_CFG_HYDROFLOW_GENERATE_DOCS="1" RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" wasm-pack build
```

### Notes on building on macOS
If you're building on macOS, you may need to install the `llvm` package with Homebrew (because the default toolchain has WASM support missing):

```bash
$ brew install llvm
```

Then, you'll need to set `TARGET_CC` and `TARGET_AR` environment variables when building the playground:

```bash
$ TARGET_CC="$(brew --prefix)/opt/llvm/bin/clang" TARGET_AR="$(brew --prefix)/opt/llvm/bin/llvm-ar" CARGO_CFG_HYDROFLOW_GENERATE_DOCS="1" RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" wasm-pack build
```

With the WASM portion built, we can launch the website with the playground loaded:

```bash
$ cd ../docs
$ LOAD_PLAYGROUND=1 npm run start
```

## Adding Papers
1. Upload the paper PDF to the `static/papers` folder.
2. Run the script `./extract-paper-thumbnails` (from this `docs` directory), which requires [ImageMagick to be installed](https://imagemagick.org/script/download.php).
3. Go to `src/pages/research.js` and add the paper to the array at the top of the file.
