# Copilot Instructions for Hydro

## Project Overview

Hydro is a high-level distributed programming framework for Rust. The repository is structured as a Cargo workspace monorepo containing multiple packages for distributed systems programming and dataflow processing.

## Repository Structure

- `dfir_rs` - Main DFIR package containing the DFIR runtime (scheduled layer)
- `dfir_lang`, `dfir_macro` - Flow syntax compiler (compiled layer)
- `hydro_lang`, `hydro_std`, `hydro_test` - Hydro functional syntax built on top of DFIR
- `hydro_deploy/` - Framework for launching Hydro programs
- `docs/` - Hydro.run website documentation
- `website_playground/` - In-browser WASM playground for compiling DFIR
- `benches/` - Microbenchmarks for DFIR and other frameworks
- `design_docs/` - Architecture design documents

General-purpose subpackages:
- `stageleft` - Framework for staged programming in Rust
- `lattices` - Abstract algebra library for lattice types
- `variadics` - Emulates variadic generics using tuple lists
- `multiplatform_test` - Convenience macro for multi-platform tests

## Rust Configuration

- **Primary Toolchain**: Rust 1.90.0 stable (specified in `rust-toolchain.toml`)
- **Edition**: 2024 (experimental edition requiring Rust 1.90.0+)
- **Required components**: rustfmt, clippy, rust-src
- **Targets**: wasm32-unknown-unknown, x86_64-unknown-linux-musl
- **Nightly Toolchain**: Used for formatting, documentation builds, and WASM tests
- The primary toolchain is automatically detected by cargo via `rust-toolchain.toml`

## Code Style and Linting

### Formatting
Formatting uses nightly Rust for the latest features:
```bash
cargo +nightly fmt --all
```

### Linting
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Workspace Lints
- `impl_trait_overcaptures` - warn
- `missing_unsafe_on_extern` - deny
- `unsafe_attr_outside_unsafe` - deny
- `unused_qualifications` - warn
- `allow_attributes` - warn
- `allow_attributes_without_reason` - warn
- `explicit_into_iter_loop` - warn
- `upper_case_acronyms` - warn

## Testing

### Running Tests
Use the `precheck.bash` script for comprehensive testing:
```bash
./precheck.bash --all        # Run all tests
./precheck.bash --dfir       # Run DFIR tests only
./precheck.bash --hydro      # Run Hydro tests only
./precheck.bash --website    # Run website/playground tests
```

### Test Types

1. **Unit and Integration Tests** - Use `cargo nextest`
2. **Doc Tests** - Run with `cargo test --doc`
3. **Wasm Tests** - For dfir_rs on wasm32-unknown-unknown target
4. **Snapshot Tests**:
   - **insta** - For DFIR graph visualizations (Mermaid/DOT format)
   - **trybuild** - For error message testing in flow syntax

### Snapshot Test Management
```bash
# Install insta CLI
cargo install cargo-insta

# Run and review snapshot tests
cargo insta test
cargo insta review

# Or use environment variables
INSTA_FORCE_PASS=1 INSTA_UPDATE=always TRYBUILD=overwrite cargo test
```

## Commit Conventions

Pull request titles and bodies must follow [Conventional Commits](https://www.conventionalcommits.org/):
- feat: New features
- fix: Bug fixes
- docs: Documentation changes
- test: Test additions/changes
- refactor: Code refactoring
- chore: Maintenance tasks

The repository uses squash+merge, so individual commits are for code review evolution only.

## Development Workflow

### Feature Branches
Create feature branches off main:
```bash
git fetch origin
git checkout -b feature/$FEATURE_NAME origin/main
git push origin HEAD
```

### Pull Request Checklist
1. Run `./precheck.bash` locally before submitting
2. Ensure all tests pass
3. Update snapshot tests if needed (review changes carefully)
4. Follow conventional commit format for PR title
5. Write descriptive PR body for changelog generation

## Special Considerations

### Snapshot Tests
- Snapshots are in `dfir_rs/tests/snapshots` (insta) and `dfir_rs/tests/compile-fail` (trybuild)
- Mermaid visualizations can be viewed at [mermaid.live](https://mermaid.live/)
- Always review snapshot changes to ensure they match expected behavior
- `precheck.bash` auto-updates snapshots - verify diffs are intentional

### WASM Support
- Node.js, wasm-bindgen-cli, and wasm-pack required for WASM tests
- Website playground uses WASM for in-browser compilation
- Special build configuration for website_playground package

### Documentation
Documentation builds use nightly Rust:
- Docs built with: `cargo +nightly doc --no-deps --all-features`
- Use `RUSTDOCFLAGS="--cfg docsrs -Dwarnings"` for strict doc builds
- Documentation available at [hydro.run](https://hydro.run/)

## Python Requirements
Some parts require Python 3.10 or later.

## Common Commands

```bash
# Format code
cargo +nightly fmt --all

# Run clippy
cargo clippy --workspace --all-targets -- -D warnings

# Run tests with nextest
cargo nextest run --workspace --all-targets --no-fail-fast

# Build docs
cargo +nightly doc --no-deps --all-features

# Check all targets without default features
cargo check --all-targets --no-default-features
```

## Release Information
See `RELEASING.md` for detailed release process and changelog generation.
