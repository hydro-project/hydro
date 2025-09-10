# WASM

## Tests

### Setup

Ensure [Node.js](https://nodejs.org/) is installed.

Install `wasm-bindgen-cli` globally: `cargo install wasm-bindgen-cli`.
`wasm-bindgen-cli` provides a test runner harness.

### Running Tests

Run (in the repo root directory):

```
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --target wasm32-unknown-unknown -p dfir_rs --tests
```

Flag explanation:

- `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner`: Tells
  Cargo to use the test harness provided by `wasm-bindgen-cli`.
- `-p hydroflow`: Only run tests in the hydroflow directory.
- `--tests`: Only run tests; do not build examples. Many (possibly all) of the
  examples in hydroflow/examples currently do not compile to WASM because they
  use networking.


### Adding a new WASM test

Instead of the `#[test]` attribute, mark the test with the
`#[multiplaform_test]` attribute.

```rust
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
fn my_test() {
  // ...
}
```
