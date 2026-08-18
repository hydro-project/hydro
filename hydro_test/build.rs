fn main() {
    hydro_build_utils::emit_nightly_configuration!();

    stageleft_tool::gen_final!();

    // TODO(shadaj): remove once the stageleft-generated ctor is Miri-compatible.
    // See https://github.com/hydro-project/stageleft/issues/84
    // The `ctor` crate's init_array entries trip Miri's ABI checks, so gate the generated
    // stageleft registration ctor behind `cfg(not(miri))`.
    let staged_deps =
        std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("staged_deps.rs");
    let contents = std::fs::read_to_string(&staged_deps).unwrap();
    std::fs::write(
        &staged_deps,
        contents.replace(
            "#[stageleft::internal::ctor::ctor",
            "#[cfg(not(miri))]\n    #[stageleft::internal::ctor::ctor",
        ),
    )
    .unwrap();

    println!("cargo:rerun-if-env-changed=MAELSTROM_PATH");
    println!("cargo::rustc-check-cfg=cfg(maelstrom_available)");
    if std::env::var("MAELSTROM_PATH").is_ok() {
        println!("cargo:rustc-cfg=maelstrom_available");
    }
}
