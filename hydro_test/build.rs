fn main() {
    hydro_build_utils::emit_nightly_configuration!();

    stageleft_tool::gen_final!();

    println!("cargo:rerun-if-env-changed=MAELSTROM_PATH");
    println!("cargo::rustc-check-cfg=cfg(maelstrom_available)");
    if std::env::var("MAELSTROM_PATH").is_ok() {
        println!("cargo:rustc-cfg=maelstrom_available");
    }
}
