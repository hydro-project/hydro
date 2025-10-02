fn main() {
    #[cfg(feature = "sim")]
    {
        println!("cargo::rerun-if-env-changed=BOLERO_FUZZER");
        if std::env::var("BOLERO_FUZZER").is_ok() {
            println!("cargo::rustc-link-arg=-export_dynamic");
        }
    }

    hydro_build_utils::emit_nightly_configuration!();
    stageleft_tool::gen_final!();
}
