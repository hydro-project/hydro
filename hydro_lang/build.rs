fn main() {
    println!("cargo:rustc-link-arg=-export_dynamic");
    hydro_build_utils::emit_nightly_configuration!();
    stageleft_tool::gen_final!();
}
