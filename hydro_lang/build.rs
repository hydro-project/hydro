fn main() {
    hydro_build_utils::emit_nightly_configuration!();
    stageleft_tool::gen_final!();

    // Capture the rustflags this crate is compiled with (every source: `RUSTFLAGS`, cargo config
    // `build.rustflags` / `target.*.rustflags`, `--config`), so the child `cargo` invocations
    // that compile generated Hydro programs can be given exactly the same flags. Cargo reruns
    // this script whenever the encoded flags change, so the baked value never goes stale.
    println!(
        "cargo::rustc-env=HYDRO_ENCODED_RUSTFLAGS={}",
        std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default()
    );
}
