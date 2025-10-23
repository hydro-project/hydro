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
}
