fn main() {
    #[cfg(feature = "sim")]
    {
        println!("cargo::rerun-if-env-changed=BOLERO_FUZZER");
        if std::env::var("BOLERO_FUZZER").is_ok() {
            #[cfg(target_os = "macos")]
            {
                println!("cargo::rustc-link-arg=-export_dynamic");
            }

            #[cfg(target_os = "linux")]
            {
                println!("cargo::rustc-link-arg=-Wl,-export-dynamic");
            }
        }
    }

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
}
