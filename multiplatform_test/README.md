# `multiplatform_test`

Provides a proc-macro that expands to testing on platforms relevant to
Hydroflow. By default, expands to testing on the host (using the normal
`#[test]` attribute) and wasm (using `#[wasm_bindgen_test]`).

For example, the test

```rust
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
fn my_test() {
  // ...
}
```

Expands to

```rust,no_run
#[test]
#[wasm_bindgen_test::wasm_bindgen_test]
fn my_test() {
  // ...
}
```

## Installation

```toml
[dependencies]
multiplatform_test = * # replace with version.
```

If you're using `wasm` naturally you will need to add the [`wasm_bindgen_test`](https://crates.io/crates/wasm-bindgen-test-macro/)
crate as a dependency.

## Usage

### Specifying platforms

There are many platforms which can be specified:
* `test` - Adds a standard [`#[test]` attribute](https://doc.rust-lang.org/reference/attributes/testing.html#the-test-attribute).
* `tokio` - Adds a [`#[tokio::test]` attribute](https://docs.rs/tokio/latest/tokio/attr.test.html).
* `async_std` - Adds an [`#[async_std::test]` attribute](https://docs.rs/async-std/latest/async_std/attr.test.html).
* `dfir` - Adds a [`#[dfir_rs::test]` attribute](https://docs.rs/hydroflow/latest/hydroflow/attr.test.html).
* `wasm` - Adds a [`#[wasm_bindgen_test::wasm_bindgen_test]` attribute](https://docs.rs/wasm-bindgen-test/0.3.36/wasm_bindgen_test/attr.wasm_bindgen_test.html).
* `env_logging` - Registers [`env_logger`](https://docs.rs/env_logger/latest/env_logger/) for [`log`ging](https://docs.rs/log/latest/log/).
* `env_tracing` - Registers a [`FmtSubscriber`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/index.html#reexport.FmtSubscriber) with an [`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) for [`tracing`](https://docs.rs/tracing/latest/tracing/).

You can test on a subset of platforms by passing in the platforms in parens:

```rust
use multiplatform_test::multiplatform_test;

#[multiplatform_test(test, env_logging)]  // Only test on the standard `#[test]` platform, but enables logging
fn my_test() {
  // ...
}
```

expands to

```rust,no_run
use multiplatform_test::multiplatform_test;

#[test]
fn my_test() {
  let _ = env_logger::builder().is_test(true).try_init();
  // ...
}
```

