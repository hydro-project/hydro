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

## Linking to rustdoc (`rust:` links)
Markdown/MDX pages can link to API documentation with the `rust:` pseudo-scheme
instead of hardcoding rustdoc URLs:

```md
[`Stream`](rust:hydro_lang::live_collections::Stream)
[`Stream::fold`](rust:hydro_lang::live_collections::Stream::fold)
[the `live_collections` module](rust:hydro_lang::live_collections)
[`setup!`](rust:hydro_lang::setup!)
```

At build time these are resolved against the compiled rustdoc in
`static/rustdoc` by `src/remark/rustdoc-links.js`. You write the full module
path of the item but never the item kind (`struct.…`, `trait.…`) or the
`.html` path — the resolver discovers those from the rustdoc output, follows
rustdoc redirect pages for re-exports, and validates associated items
(`Type::method`) against the anchors on the generated page, so stale links
fail the production build instead of 404ing. In the rare case of a same-name
collision, disambiguate rustdoc-style with a `kind@` prefix (`struct@`,
`trait@`, `fn@`, `mod@`, ...) or a trailing `()` / `!`.

In local dev, if `static/rustdoc` doesn't exist, `rust:` links degrade to
rustdoc search URLs with a console warning. Production builds
(`NODE_ENV=production`, as in `build_docs.bash`) require compiled rustdoc and
fail on any unresolvable link. To resolve links properly during local dev:

```bash
$ cargo doc --no-deps --all-features   # at the repo root
$ ln -sfn ../../target/doc static/rustdoc  # in this docs directory
```

## Building the DFIR docs
`dfir/syntax/surface_ops_gen.md` is generated during the Rust build.
If you have not built the Rust project yet, you may need to run this command generate or update it:
```bash
$ cargo build -p dfir_macro -p dfir_lang
```
(The `DFIR_GENERATE_DOCS` environment variable must be set for this, but `.cargo/config.toml` already does this for us).

## Building the Playground
By default, the DFIR / Datalog playgrounds are not loaded when launching the website. To build the playground, you'll need to follow a couple additional steps. This requires Rust and [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```bash
$ rustup target add wasm32-unknown-unknown
$ cargo install wasm-pack
$ cd ../website_playground
$ RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" wasm-pack build
```

### Notes on building on macOS
If you're building on macOS, you may need to install the `llvm` package with Homebrew (because the default toolchain has WASM support missing):

```bash
$ brew install llvm
```

Then, you'll need to set `TARGET_CC` and `TARGET_AR` environment variables when building the playground:

```bash
$ TARGET_CC="$(brew --prefix)/opt/llvm/bin/clang" TARGET_AR="$(brew --prefix)/opt/llvm/bin/llvm-ar" RUSTFLAGS="--cfg procmacro2_semver_exempt --cfg super_unstable" wasm-pack build
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
