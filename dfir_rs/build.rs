use rustc_version::{Channel, version_meta};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(nightly)");
    if matches!(
        version_meta().map(|meta| meta.channel),
        Ok(Channel::Nightly)
    ) {
        println!("cargo:rustc-cfg=nightly");
    }
}
