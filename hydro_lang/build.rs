fn main() {
    hydro_build_utils::emit_nightly_configuration!();
    stageleft_tool::gen_final!();

    // Capture the rustflags this crate is compiled with (every source: `RUSTFLAGS`, cargo config
    // `build.rustflags` / `target.*.rustflags`, `--config`), so the child `cargo` invocations
    // that compile generated Hydro programs can be given exactly the same flags. Cargo already
    // reruns this script whenever the flags change (they are part of the build-script unit's
    // fingerprint); the `rerun-if-env-changed` line declares the dependency explicitly and also
    // covers `CARGO_ENCODED_RUSTFLAGS` being set in the outer environment by hand.
    println!("cargo::rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!(
        "cargo::rustc-env=HYDRO_ENCODED_RUSTFLAGS={}",
        std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default()
    );
}
