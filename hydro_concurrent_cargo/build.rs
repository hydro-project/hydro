fn main() {
    println!("cargo::rustc-check-cfg=cfg(new_build_dir_layout)");

    // Detect which build-dir layout the cargo building this crate uses, by
    // inspecting the shape of our own OUT_DIR:
    //
    // - legacy layout: `<target>/debug/build/<pkg>-<16 hex>/out`
    // - new layout (`-Zbuild-dir-new-layout`, default on current nightly):
    //   `<build-dir>/debug/build/<pkg>/<16 hex>/out`
    //
    // i.e. under the new layout the parent of `out/` is a bare hex hash, while
    // under the legacy layout it is always `<pkg>-<hash>` (which contains a
    // `-`). This observes the actual layout rather than a channel/version
    // proxy, so it stays correct when the new layout reaches stable or when it
    // is toggled explicitly.
    //
    // The runtime cargo invocations coordinated by this crate use the same
    // toolchain that compiled it (rustup pins the toolchain for the whole
    // process tree), so a compile-time verdict is sound; switching toolchains
    // recompiles this crate and re-runs this detection.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);
    let is_new_layout = out_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .is_some_and(|name| {
            name.len() == 16
                && name
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        });
    if is_new_layout {
        println!("cargo::rustc-cfg=new_build_dir_layout");
    }
}
